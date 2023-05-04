use crate::cubemx::ip::dma::{self, Parameter};
use anyhow::{anyhow, Chain, Context, Result};
use std::collections::{BTreeMap, BTreeSet, HashMap};

//pub type Map = BTreeMap<String, BTreeMap<(u8, u8), BTreeMap<String, BTreeSet<String>>>>;
pub type Map = BTreeMap<String, BTreeMap<(u8, u8), BTreeSet<String>>>;

pub fn ip_to_table(ip: &dma::Ip) -> Result<Map> {
    let modemap: Result<BTreeMap<_, _>> = ip
        .modes
        .iter()
        .filter(|refmode| refmode.basemode == "DMA_Request" && refmode.name != "MEMTOMEM")
        .map(|refmode| {
            let name = refmode.name.clone();
            let mut channel = None;
            let mut direction = Vec::new();
            let mut mode = Vec::new();
            let mut instance = Vec::new();
            for param in &refmode.parameters {
                match param.name.as_str() {
                    "Channel" => {
                        for val in &param.possible_values {
                            assert!(channel.is_none());
                            assert!(val.starts_with("DMA_CHANNEL_"));
                            channel = Some(val[12..].parse().unwrap());
                        }
                    }
                    "Direction" => {
                        direction.extend(param.possible_values.iter().cloned());
                    }
                    "Mode" => {
                        mode.extend(param.possible_values.iter().cloned());
                    }
                    "IpInstance" => {
                        instance.extend(param.possible_values.iter().cloned());
                    }
                    _ => {}
                }
            }
            let Some(channel) = channel else {
            return Err(anyhow!("Channel is absent in `{name}`"));
        };
            Ok((
                name,
                Mode {
                    channel,
                    direction,
                    mode,
                    instance,
                },
            ))
        })
        .collect();
    let modemap = modemap?;

    let mut map: Map = BTreeMap::new();
    for dma in &ip.dmas {
        for mode in &dma.modes {
            let dma_name = &mode.name;
            assert!(dma_name.starts_with("DMA"));
            for operator in &mode.operators {
                for stream in &operator.modes {
                    let stream_name = &stream.name;
                    assert!(stream_name.starts_with(&format!("{dma_name}_Stream")));
                    let stream_id = stream_name[11..].parse()?;
                    for operator in &stream.operators {
                        for mode in &operator.modes {
                            let mode_name = &mode.name;
                            if mode_name == "MEMTOMEM" {
                                continue;
                            }
                            let m = modemap.get(mode_name).unwrap();
                            map.entry(dma.name.clone())
                                .or_default()
                                .entry((stream_id, m.channel))
                                .or_default()
                                //.entry(mode_name.to_string())
                                //.or_default()
                                //.insert(ip.version.clone());
                                .insert(mode_name.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(map)
}

pub struct Mode {
    pub channel: u8,
    pub direction: Vec<String>,
    pub mode: Vec<String>,
    pub instance: Vec<String>,
}
