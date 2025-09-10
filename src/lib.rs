use std::{collections::HashMap, net::{Ipv4Addr, Ipv6Addr}, time::Duration};

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::{net::UdpSocket, time::timeout};
use hickory_proto::op::{Message, MessageType, OpCode, ResponseCode};
use hickory_proto::rr::{Name, RData, Record, RecordType};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub listen: Option<Listen>,
    pub upstream: Option<Vec<String>>, // list of IPs or hostnames
    pub default_ttl: Option<u32>,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Listen { pub host: String, pub port: u16 }

#[derive(Debug, Deserialize, Clone)]
pub struct Zone {
    pub origin: String,
    pub ttl: Option<u32>,
    pub soa: Soa,
    pub ns: Vec<String>,
    pub records: Vec<RecordCfg>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Soa {
    pub mname: String,
    pub rname: String,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RecordCfg {
    pub name: String,     // relative label or '@'
    #[serde(rename = "type")]
    pub r#type: String,   // A, AAAA, CNAME, TXT, MX, NS, SOA
    pub value: Option<String>,
    pub ttl: Option<u32>,
    // MX specific
    pub preference: Option<u16>,
}

#[derive(Clone)]
pub struct ZoneStore {
    zones: Vec<ZoneData>,
}

#[derive(Clone)]
struct ZoneData {
    origin: Name,
    default_ttl: u32,
    soa: Soa,
    ns: Vec<Name>,
    // exact records: fqdn -> type -> Vec<RData>
    exact: HashMap<Name, HashMap<RecordType, Vec<(RData, u32)>>>,
    // wildcard records: suffix (e.g., "wild.example.") -> type -> Vec<RData>
    wild: HashMap<Name, HashMap<RecordType, Vec<(RData, u32)>>>,
}

pub fn ensure_fqdn(s: &str) -> String { if s.ends_with('.') { s.to_lowercase() } else { format!("{}.", s.to_lowercase()) } }

pub fn parse_name(s: &str) -> Name { Name::from_ascii(&ensure_fqdn(s)).expect("invalid name in config") }

pub fn value_to_name(value: &str, origin: &Name) -> Name {
    if value == "@" { return origin.clone(); }
    if value.ends_with('.') { return parse_name(value); }
    parse_name(&format!("{}.{}", value, origin))
}

impl ZoneStore {
    pub fn from_config(cfg: &Config) -> Result<Self> {
        let default_ttl = cfg.default_ttl.unwrap_or(300);
        let mut zones = Vec::new();
        for z in &cfg.zones {
            let origin = parse_name(&z.origin);
            let zone_default_ttl = z.ttl.unwrap_or(default_ttl);
            let ns: Vec<Name> = z.ns.iter().map(|n| value_to_name(n, &origin)).collect();
            let mut zd = ZoneData {
                origin: origin.clone(),
                default_ttl: zone_default_ttl,
                soa: z.soa.clone(),
                ns,
                exact: HashMap::new(),
                wild: HashMap::new(),
            };

            for rc in &z.records {
                let ttl = rc.ttl.unwrap_or(zone_default_ttl);
                let rtype = parse_rtype(&rc.r#type)?;
                let rdata = build_rdata(rtype, rc, &origin)?;

                if rc.name == "@" {
                    let fqdn = origin.clone();
                    zd.exact.entry(fqdn).or_default().entry(rtype).or_default().push((rdata, ttl));
                } else if rc.name.starts_with("*.") {
                    let suffix_rel = rc.name.trim_start_matches("*.");
                    let full = if suffix_rel.ends_with('.') { suffix_rel.to_string() } else { format!("{}.{}", suffix_rel, origin) };
                    let suffix_name = parse_name(&full);
                    zd.wild.entry(suffix_name).or_default().entry(rtype).or_default().push((rdata, ttl));
                } else {
                    let full = if rc.name.ends_with('.') { rc.name.clone() } else { format!("{}.{}", rc.name, origin) };
                    let fqdn = parse_name(&full);
                    zd.exact.entry(fqdn).or_default().entry(rtype).or_default().push((rdata, ttl));
                }
            }

            zones.push(zd);
        }
        Ok(Self { zones })
    }

    fn find_zone(&self, qname: &Name) -> Option<&ZoneData> {
        let q = qname.to_ascii();
        self.zones
            .iter()
            .filter(|z| { let zo = z.origin.to_ascii(); q == zo || q.ends_with(&zo) })
            .max_by_key(|z| z.origin.num_labels())
    }

    pub fn answer(&self, req: &Message) -> Option<Message> {
        let mut resp = Message::new();
        resp.set_id(req.id());
        resp.set_message_type(MessageType::Response);
        resp.set_op_code(OpCode::Query);
        resp.set_recursion_desired(req.recursion_desired());
        resp.set_recursion_available(false);

        if let Some(q) = req.queries().first() {
            let qname = q.name().clone();
            let qtype = q.query_type();
            resp.add_query(q.clone());
            if let Some(zone) = self.find_zone(&qname) {
                resp.set_authoritative(true);
                let mut answered = false;
                if let Some(tmap) = zone.exact.get(&qname) {
                    if let Some(records) = tmap.get(&qtype) {
                        for (rd, ttl) in records { resp.add_answer(record(&qname, *ttl, rd.clone())); }
                        answered = true;
                    } else if qtype != RecordType::CNAME {
                        if let Some(cnames) = tmap.get(&RecordType::CNAME) {
                            for (rd, ttl) in cnames {
                                resp.add_answer(record(&qname, *ttl, rd.clone()));
                                if let RData::CNAME(target) = rd {
                                    if let Some((tmap2, z2)) = self.lookup_exact(&target) {
                                        if let Some(addl) = tmap2.get(&qtype) { for (rd2, ttl2) in addl { resp.add_answer(record(&target, *ttl2, rd2.clone())); } }
                                        add_ns_soa(&mut resp, z2);
                                    }
                                }
                            }
                            answered = true;
                        }
                    }
                }
                if !answered {
                    if let Some(tmap) = self.find_wildcard(&qname, zone) {
                        if let Some(records) = tmap.get(&qtype) {
                            for (rd, ttl) in records { resp.add_answer(record(&qname, *ttl, rd.clone())); }
                            answered = true;
                        } else if qtype != RecordType::CNAME {
                            if let Some(cnames) = tmap.get(&RecordType::CNAME) {
                                for (rd, ttl) in cnames { resp.add_answer(record(&qname, *ttl, rd.clone())); }
                                answered = true;
                            }
                        }
                    }
                }

                if answered { add_ns_soa(&mut resp, zone); resp.set_response_code(ResponseCode::NoError); }
                else { resp.set_response_code(ResponseCode::NoError); add_ns_soa(&mut resp, zone); }
                return Some(resp);
            }
        }
        None
    }

    fn lookup_exact(&self, name: &Name) -> Option<(&HashMap<RecordType, Vec<(RData, u32)>>, &ZoneData)> {
        if let Some(zone) = self.find_zone(name) { if let Some(tmap) = zone.exact.get(name) { return Some((tmap, zone)); } }
        None
    }

    fn find_wildcard<'a>(&'a self, qname: &Name, zone: &'a ZoneData) -> Option<&'a HashMap<RecordType, Vec<(RData, u32)>>> {
        let q = qname.to_ascii();
        let z = zone.origin.to_ascii();
        if !q.ends_with(&z) { return None; }
        let q_no_dot = q.trim_end_matches('.');
        let z_no_dot = z.trim_end_matches('.');
        let qparts: Vec<&str> = q_no_dot.split('.').collect();
        let zparts: Vec<&str> = z_no_dot.split('.').collect();
        if qparts.len() <= zparts.len() { return None; }
        for i in 1..=(qparts.len() - zparts.len()) {
            let suffix = format!("{}.", qparts[i..].join("."));
            let suffix_name = parse_name(&suffix);
            if let Some(tmap) = zone.wild.get(&suffix_name) { return Some(tmap); }
        }
        None
    }
}

fn add_ns_soa(resp: &mut Message, zone: &ZoneData) {
    let soa = &zone.soa;
    let mname = parse_name(&soa.mname);
    let rname = parse_name(&soa.rname);
    let refresh = (soa.refresh.min(i32::MAX as u32)) as i32;
    let retry = (soa.retry.min(i32::MAX as u32)) as i32;
    let expire = (soa.expire.min(i32::MAX as u32)) as i32;
    let rdata = RData::SOA(hickory_proto::rr::rdata::SOA::new(mname, rname, soa.serial, refresh, retry, expire, soa.minimum));
    let soa_rec = Record::from_rdata(zone.origin.clone(), zone.default_ttl, rdata);
    resp.add_name_server(soa_rec);
    for ns in &zone.ns {
        let ns_rdata = RData::NS(hickory_proto::rr::rdata::NS(ns.clone()));
        let ns_rec = Record::from_rdata(zone.origin.clone(), zone.default_ttl, ns_rdata);
        resp.add_name_server(ns_rec);
    }
}

fn record(name: &Name, ttl: u32, rdata: RData) -> Record<RData> {
    Record::from_rdata(name.clone(), ttl, rdata)
}

fn parse_rtype(s: &str) -> Result<RecordType> {
    let t = match s.to_uppercase().as_str() {
        "A" => RecordType::A,
        "AAAA" => RecordType::AAAA,
        "CNAME" => RecordType::CNAME,
        "TXT" => RecordType::TXT,
        "MX" => RecordType::MX,
        "NS" => RecordType::NS,
        "SOA" => RecordType::SOA,
        other => anyhow::bail!("unsupported record type in config: {}", other),
    }; Ok(t)
}

fn build_rdata(rt: RecordType, rc: &RecordCfg, origin: &Name) -> Result<RData> {
    Ok(match rt {
        RecordType::A => { let ip: Ipv4Addr = rc.value.as_ref().context("A record requires value")?.parse()?; RData::A(ip.into()) }
        RecordType::AAAA => { let ip: Ipv6Addr = rc.value.as_ref().context("AAAA record requires value")?.parse()?; RData::AAAA(ip.into()) }
        RecordType::CNAME => { let n = rc.value.as_ref().context("CNAME requires value")?; RData::CNAME(hickory_proto::rr::rdata::CNAME(value_to_name(n, origin))) }
        RecordType::NS => { let n = rc.value.as_ref().context("NS requires value")?; RData::NS(hickory_proto::rr::rdata::NS(value_to_name(n, origin))) }
        RecordType::TXT => { let v = rc.value.as_ref().context("TXT requires value")?.clone(); RData::TXT(hickory_proto::rr::rdata::TXT::new(vec![v])) }
        RecordType::MX => { let pref = rc.preference.context("MX requires preference")?; let n = rc.value.as_ref().context("MX requires value")?; RData::MX(hickory_proto::rr::rdata::MX::new(pref, value_to_name(n, origin))) }
        RecordType::SOA => anyhow::bail!("SOA should be specified in zone.soa, not in records"),
        _ => anyhow::bail!("unsupported type: {:?}", rt),
    })
}

pub async fn forward_udp(upstream: &str, data: &[u8]) -> Result<Vec<u8>> {
    let sock = UdpSocket::bind("0.0.0.0:0").await?;
    sock.connect(upstream).await?;
    sock.send(data).await?;
    let mut buf = vec![0u8; 4096];
    let (len, _) = timeout(Duration::from_secs(3), sock.recv_from(&mut buf)).await??;
    Ok(buf[..len].to_vec())
}
