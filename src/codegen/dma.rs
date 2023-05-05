use crate::cubemx::ip::dma::{self, Parameter};
use anyhow::{anyhow, Chain, Context, Result};
use std::{collections::{BTreeMap, BTreeSet, HashMap}, fmt::format};

//pub type Map = BTreeMap<String, BTreeMap<(u8, u8), BTreeMap<String, BTreeSet<String>>>>;
pub type Map = BTreeMap<String, BTreeMap<(u8, u8), BTreeMap<String, Mode>>>;


pub fn print_table(maps: &BTreeMap<String, Map>) {
    for (target, map) in maps {
        println!("#[cfg(feature = \"{target}\")]");
        println!("dma_map {{");
        for (dma, x) in map {
            for ((s, c), xx) in x {
                for (modename, mode) in xx {
                    let dirs = mode.direction.join(" | ");
                    //let bb = mode.mode.join(" | ");
                    println!("(Stream{s}<{dma}>:{c}, {modename}, [{dirs}]),");
                }
            }
        }
        println!("}}");
    }
}

pub fn ip_to_table(ip: &dma::Ip) -> Result<(String, Map)> {
    let mut modemap = BTreeMap::new();
    for refmode in &ip.modes {
        if !(refmode.basemode == "DMA_Request" && refmode.name != "MEMTOMEM") {
            continue;
        }
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
        let channel = if let Some(channel) = channel {
            channel
        } else {
            println!("Channel is absent in `{name}`");
            continue;
        };
        modemap.insert(
            name,
            Mode {
                channel,
                direction,
                mode,
                instance,
            },
        );
    }

    let mut map: Map = BTreeMap::new();
    for dma in &ip.dmas {
        for mode in &dma.modes {
            let dma_name = &mode.name;
            assert!(dma_name.starts_with("DMA"));
            for operator in &mode.operators {
                for stream in &operator.modes {
                    let stream_name = &stream.name;
                    if !stream_name.starts_with(&format!("{dma_name}_Stream")) {
                        return Err(anyhow!("Stream name {stream_name} is incorrect"));
                    }
                    let stream_id = stream_name[11..].parse()?;
                    for operator in &stream.operators {
                        for mode in &operator.modes {
                            let mode_name = &mode.name;
                            if mode_name == "MEMTOMEM" {
                                continue;
                            }
                            let m = modemap.get(mode_name).unwrap();
                            map.entry(dma_name.clone())
                                .or_default()
                                .entry((stream_id, m.channel))
                                .or_default()
                                .insert(mode_name.to_string(), m.clone());
                        }
                    }
                }
            }
        }
    }
    Ok((ip.version.clone(), map))
}

#[derive(Clone, Debug)]
pub struct Mode {
    pub channel: u8,
    pub direction: Vec<String>,
    pub mode: Vec<String>,
    pub instance: Vec<String>,
}
