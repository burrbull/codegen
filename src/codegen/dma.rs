use crate::cubemx::ip::dma;
use anyhow::{anyhow, Result};
use std::{collections::BTreeMap, fmt::Display, str::FromStr};

pub type Map = BTreeMap<String, BTreeMap<(CR, SC), BTreeMap<String, Mode>>>;

pub fn print_table(maps: &BTreeMap<String, Map>) {
    for (target, map) in maps {
        println!("#[cfg(feature = \"{target}\")]");
        println!("dma_map! {{");
        for (dma, x) in map {
            for ((c, s), xx) in x {
                for (modename, mode) in xx {
                    let dirs = mode.direction.iter().map(ToString::to_string).collect::<Vec<_>>().join(" | ");
                    //let bb = mode.mode.join(" | ");
                    match (s, c) {
                        (SC::Stream(s), CR::Channel(c)) => {
                            println!(
                                "    (Stream{s}<{dma}>:{c}, {modename}, [{dirs}]), //{modename}"
                            );
                        }
                        (SC::Channel(s), CR::Request(c)) => {
                            println!(
                                "    (Channel{s}<{dma}>:{c}, {modename}, [{dirs}]), //{modename}"
                            );
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        println!("}}");
    }
}

pub fn get_mode_maps(ip: &dma::Ip) -> BTreeMap<String, Mode> {
    let mut modemap = BTreeMap::new();
    for refmode in &ip.modes {
        if !(refmode.basemode == "DMA_Request" && refmode.name != "MEMTOMEM") {
            continue;
        }
        let name = refmode.name.clone();
        let mut channel = None;
        let mut request = None;
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
                "Request" => {
                    for val in &param.possible_values {
                        assert!(request.is_none());
                        assert!(val.starts_with("DMA_REQUEST_"));
                        request = Some(val[12..].parse().unwrap());
                    }
                }
                "Direction" => {
                    for d in &param.possible_values {
                        let d = Direction::from_str(d).unwrap();
                        direction.push(d);
                    }
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

        let cr = if let Some(channel) = channel {
            CR::Channel(channel)
        } else if let Some(request) = request {
            CR::Request(request)
        } else {
            println!("Channel is absent in `{name}`");
            continue;
        };
        modemap.insert(
            name,
            Mode {
                cr,
                direction,
                mode,
                instance,
            },
        );
    }
    modemap
}

pub fn ip_to_table(ip: &dma::Ip) -> Result<(String, Map)> {
    let modemap = get_mode_maps(ip);
    let mut map: Map = BTreeMap::new();
    for dma in &ip.dmas {
        for mode in &dma.modes {
            let dma_name = &mode.name;
            assert!(dma_name.starts_with("DMA"));
            for operator in &mode.operators {
                for stream in &operator.modes {
                    let stream_name = &stream.name;
                    let stream_id = if stream_name.starts_with(&format!("{dma_name}_Stream")) {
                        SC::Stream(stream_name[11..].parse()?)
                    } else if stream_name.starts_with(&format!("{dma_name}_Channel")) {
                        SC::Channel(stream_name[12..].parse()?)
                    } else {
                        return Err(anyhow!("Stream/Channel name {stream_name} is incorrect"));
                    };
                    for operator in &stream.operators {
                        for mode in &operator.modes {
                            let mode_name = &mode.name;
                            if mode_name == "MEMTOMEM" {
                                continue;
                            }
                            let m = modemap.get(mode_name).expect("Missing entry for {mode_name}");
                            map.entry(dma_name.clone())
                                .or_default()
                                .entry((m.cr, stream_id))
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
    pub cr: CR,
    pub direction: Vec<Direction>,
    pub mode: Vec<String>,
    pub instance: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CR {
    Channel(u8),
    Request(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SC {
    Stream(u8),
    Channel(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Direction {
    PtM,
    MtP,
    MtM
}

impl FromStr for Direction {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "DMA_PERIPH_TO_MEMORY" => Ok(Self::PtM),
            "DMA_MEMORY_TO_PERIPH" => Ok(Self::MtP),
            "DMA_MEMORY_TO_MEMORY" => Ok(Self::MtM),
            _ => Err(()),
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PtM => f.write_str("PeripheralToMemory"),
            Self::MtP => f.write_str("MemoryToPeripheral"),
            Self::MtM => f.write_str("MemoryToMemory"),
        }
    }
}