use crate::cubemx::ip::gpio;
use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet, HashMap};

struct Port<'a> {
    id: char,
    pins: Vec<&'a gpio::Pin>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pin(char, u8, u8);

pub type XMap = BTreeMap<
    String, // peripheral
    BTreeMap<
        String, // alt function
        BTreeMap<
            Pin,              // pin
            BTreeSet<String>, // features
        >,
    >,
>;

#[derive(Clone, Copy)]
enum Otype {
    Fixed(&'static str),
    Default(&'static str),
}

pub fn gen_mappings(gpio_ips: &[gpio::Ip]) -> Result<()> {
    let mut all_macros = Vec::<PortMacro>::new();
    let mut map = BTreeMap::new();
    for ip in gpio_ips.iter() {
        println!();
        let ms = gen_gpio_ip(&mut map, ip)?;
        for m in ms.into_iter() {
            let mut same = None;
            for (i, am) in all_macros.iter().enumerate() {
                if m.string == am.string {
                    same = Some(i);
                    break;
                }
            }
            if let Some(i) = same {
                all_macros[i].features.extend(m.features);
            } else {
                all_macros.push(m);
            }
        }
    }
    let mut allmacros = Vec::<PortMacro>::new();
    for m in all_macros.into_iter() {
        let mut same = None;
        for (i, am) in allmacros.iter().enumerate() {
            if m.features == am.features {
                same = Some(i);
                break;
            }
        }
        if let Some(i) = same {
            allmacros[i].string.push('\n');
            allmacros[i].string.push_str(&m.string);
        } else {
            allmacros.push(m);
        }
    }
    /*for m in allmacros {
        println!("{m}");
    }*/

    let mut series: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    let mut results = String::new();
    for (per, x) in &map {
        let mut all_features = BTreeSet::<String>::new();
        for (_, xx) in x {
            for (_, fs) in xx {
                all_features.extend(fs.iter().cloned());
                for s in fs {
                    series.entry(s.into()).or_default().insert(per.into());
                }
            }
        }
        results.push_str(&print_features(
            &all_features.iter().collect::<Vec<_>>(),
            "",
        )?);
        results.push_str(&format!(
            r#"pub mod {per} {{
    use super::*;

    pin! {{
"#
        ));
        for (alt, xx) in x {
            let mut otype = None;
            if per.starts_with("uart") || per.starts_with("usart") {
                if alt == "Tx" || alt == "Rx" {
                    otype = Some(Otype::Default("PushPull"));
                } else {
                    otype = Some(Otype::Fixed("PushPull"));
                }
            } else if per.starts_with("tim") {
                if alt.starts_with("Ch") {
                    otype = Some(Otype::Default("PushPull"));
                } else {
                    otype = Some(Otype::Fixed("PushPull"));
                }
            } else if per == "ir" {
                if alt == "Out" {
                    otype = Some(Otype::Default("PushPull"));
                }
            } else if per.starts_with("lptim") {
                if alt == "Out" {
                    otype = Some(Otype::Default("PushPull"));
                } else {
                    otype = Some(Otype::Fixed("PushPull"));
                }
            } else if per.starts_with("comp") {
                if alt == "Out" {
                    otype = Some(Otype::Fixed("PushPull"));
                }
            } else if per.starts_with("i2c") || per.starts_with("fmpi2c") {
                otype = Some(Otype::Fixed("OpenDrain"));
            } else if per.starts_with("can")
                || per.starts_with("fdcan")
                || per.starts_with("dcmi")
                || per.starts_with("dfsdm")
                || per.starts_with("dsihost")
                || per.starts_with("eth")
                || per.starts_with("fmc")
                || per.starts_with("fsmc")
                || per.starts_with("i2s")
                || per.starts_with("ltdc")
                || per.starts_with("mdios")
                || per.starts_with("quadspi")
                || per.starts_with("octospi")
                || per.starts_with("pssi")
                || per.starts_with("rcc")
                || per.starts_with("rtc")
                || per.starts_with("sai")
                || per.starts_with("sdio")
                || per.starts_with("spi")
                || per.starts_with("swpmi")
                || per.starts_with("sys")
                || per.starts_with("sdmmc")
                || per.starts_with("spdifrx")
                || per.starts_with("tsc")
                || per.starts_with("usb")
            {
                otype = Some(Otype::Fixed("PushPull"));
            }
            let mut nopin = false;
            if per.starts_with("uart") || per.starts_with("usart") || per.starts_with("can") {
                if alt == "Tx" || alt == "Rx" {
                    nopin = true;
                }
            } else if per.starts_with("spi") {
                if alt == "Miso" || alt == "Mosi" || alt == "Sck" {
                    nopin = true;
                }
            } else if per.starts_with("i2s") {
                if alt == "Mck" {
                    nopin = true;
                }
            }
            let fixed = if let Some(Otype::Fixed(otype)) = otype {
                format!(", {otype}")
            } else {
                String::new()
            };
            let default = if let Some(Otype::Default(otype)) = otype {
                format!(" default:{otype}")
            } else {
                String::new()
            };
            let nopin = if nopin {
                format!("no:NoPin, ")
            } else {
                String::new()
            };
            results.push_str(&format!(
                "        <{alt}{}>{} for {}[\n",
                fixed, default, nopin
            ));
            for (pin, features) in xx {
                let mut diff = all_features.difference(&features);
                if !(pin.0 == 'I' && pin.1 == 8) {
                    for f in features {
                        series
                            .entry(f.into())
                            .or_default()
                            .insert(format!("gpio{}", pin.0.to_lowercase()));
                    }
                }
                let features = if diff.next().is_some() {
                    all_features.intersection(&features).collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                results.push_str(&print_features(&features, "            ")?);
                let pin = format!("P{}{}<{}>", pin.0, pin.1, pin.2);
                results.push_str(&format!("            {pin},\n\n"));
            }
            results.push_str(&format!("        ],\n\n"));
        }

        results.push_str("    }\n");
        if per.starts_with("tim") {
            results.push_str(&format!(
                "\n    use crate::pac::{} as TIM;\n    ",
                per.to_uppercase()
            ));
            for i in 1..=4 {
                if x.contains_key(&format!("Ch{i}")) {
                    results.push_str(&format!(
                        r#"impl TimCPin<C{i}> for TIM {{
        type Ch<Otype> = Ch{i}<Otype>;
    }}
    "#
                    ));
                }
                if x.contains_key(&format!("Ch{i}N")) {
                    results.push_str(&format!(
                        r#"impl TimNCPin<C{i}> for TIM {{
        type ChN<Otype> = Ch{i}N<Otype>;
    }}
    "#
                    ));
                }
            }
            if x.contains_key("Bkin") {
                results.push_str(
                    r#"impl TimBkin for TIM {
        type Bkin = Bkin;
    }
    "#,
                );
            }
            if x.contains_key("Etr") {
                results.push_str(
                    r#"impl TimEtr for TIM {
        type Etr = Etr;
    }
    "#,
                );
            }
        }

        if per.starts_with("spi") {
            results.push_str(&format!(
                r#"    impl SpiCommon for crate::pac::{} {{
        type Miso = Miso;
        type Mosi = Mosi;
        type Nss = Nss;
        type Sck = Sck;
    }}
    "#,
                per.to_uppercase()
            ));
        }

        if per.starts_with("can") {
            results.push_str(&format!(
                r#"    impl CanCommon for crate::pac::{} {{
        type Rx = Rx;
        type Tx = Tx;
    }}
    "#,
                per.to_uppercase()
            ));
        }
        if per.starts_with("i2c") || per.starts_with("fmpi2c") {
            results.push_str(&format!(
                r#"    use crate::pac::{} as I2C;
            impl I2cCommon for I2C {{
                type Scl = Scl;
                type Sda = Sda;
                type Smba = Smba;
            }}
    "#,
                per.to_uppercase()
            ));
        }

        if per.starts_with("sai") {
            results.push_str(&format!(
                "    use crate::pac::{} as SAI;\n",
                per.to_uppercase()
            ));
            results.push_str(
                r##"    pub struct ChannelA;
    pub struct ChannelB;
    impl SaiChannels for SAI {
        type A = ChannelA;
        type B = ChannelB;
    }
    impl SaiChannel for ChannelA {
        type Fs = FsA;
        type Mclk = MclkA;
        type Sck = SckA;
        type Sd = SdA;
    }
    impl SaiChannel for ChannelB {
        type Fs = FsB;
        type Mclk = MclkB;
        type Sck = SckB;
        type Sd = SdB;
    }
    "##,
            );
        }

        if per.starts_with("spdifrx") {
            results.push_str(
                r##"
    use crate::pac::SPDIFRX;
    impl SPdifIn<0> for SPDIFRX {
        type In = In0;
    }
    impl SPdifIn<1> for SPDIFRX {
        type In = In1;
    }
    impl SPdifIn<2> for SPDIFRX {
        type In = In2;
    }
    impl SPdifIn<3> for SPDIFRX {
        type In = In3;
    }
    "##,
            );
        }

        if per.starts_with("usart") {
            results.push_str(&format!(
                "    use crate::pac::{} as USART;\n",
                per.to_uppercase()
            ));
            results.push_str(
                r##"    impl SerialAsync for USART {
        type Rx<Otype> = Rx<Otype>;
        type Tx<Otype> = Tx<Otype>;
    }
    "##,
            );
            if x.contains_key("Ck") {
                results.push_str(
                    r#"impl SerialSync for USART {
            type Ck = Ck;
        }
    "#,
                );
            }
            if x.contains_key("Cts") {
                results.push_str(
                    r#"impl SerialRs232 for USART {
            type Cts = Cts;
            type Rts = Rts;
        }
    "#,
                );
            }
        }

        if per.starts_with("uart") {
            results.push_str(&format!(
                "    use crate::pac::{} as UART;\n",
                per.to_uppercase()
            ));
            results.push_str(
                r##"    impl SerialAsync for UART {
        type Rx<Otype> = Rx<Otype>;
        type Tx<Otype> = Tx<Otype>;
    }
    "##,
            );
            if x.contains_key("Ck") {
                results.push_str(
                    r#"impl SerialSync for UART {
            type Ck = Ck;
        }
    "#,
                );
            }
            if x.contains_key("Cts") {
                results.push_str(
                    r#"impl SerialRs232 for UART {
            type Cts = Cts;
            type Rts = Rts;
        }
    "#,
                );
            }
        }

        results.push_str("}\n\n");
    }
    //println!("{results}");
    let mut results = String::new();
    for (s, pers) in series {
        results.push_str(&format!("{s} = [\n    "));
        for p in pers {
            results.push_str(&format!("\"{p}\", "));
        }
        results.push_str("\n]\n");
    }
    println!("{results}");
    Ok(())
}

fn print_features(features: &[&String], tab: &str) -> anyhow::Result<String> {
    use std::fmt::Write;
    let mut f = String::new();
    if features.is_empty() {
    } else if features.len() == 1 {
        writeln!(f, r#"{tab}#[cfg(feature = "{}")]"#, features[0])?;
    } else if features.len() < 4 {
        writeln!(
            f,
            "{tab}#[cfg(any({}))]",
            features
                .iter()
                .map(|s| format!(r#"feature = "{s}""#))
                .collect::<Vec<_>>()
                .join(", ")
        )?;
    } else {
        writeln!(
            f,
            "{tab}#[cfg(any(\n    {tab}{}\n{tab}))]",
            features
                .iter()
                .map(|s| format!(r#"feature = "{s}""#))
                .collect::<Vec<_>>()
                .join(&format!(",\n    {tab}"))
        )?;
    }
    Ok(f)
}

fn gen_gpio_ip(map: &mut XMap, ip: &gpio::Ip) -> Result<Vec<PortMacro>> {
    let feature = ip_version_to_feature(&ip.version)?;
    let ports = merge_pins_by_port(&ip.pins)?;

    let mut macros = Vec::new();
    for port in &ports {
        for string in gen_port(&feature, map, port)?.into_iter() {
            macros.push(PortMacro {
                features: vec![feature.clone()],
                string,
            });
        }
    }
    Ok(macros)
}

fn ip_version_to_feature(ip_version: &str) -> Result<String> {
    static VERSION: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^STM32(?P<version>\w+)_gpio_v1_0$").unwrap());

    let captures = VERSION
        .captures(ip_version)
        .with_context(|| format!("invalid GPIO IP version: {}", ip_version))?;

    let version = captures.name("version").unwrap().as_str();
    let feature = format!("gpio-{}", version.to_lowercase());
    Ok(feature)
}

fn merge_pins_by_port(pins: &[gpio::Pin]) -> Result<Vec<Port>> {
    let mut pins_by_port = HashMap::new();
    for pin in pins.iter() {
        pins_by_port
            .entry(pin.port()?)
            .and_modify(|e: &mut Vec<_>| e.push(pin))
            .or_insert_with(|| vec![pin]);
    }

    let mut ports = Vec::new();
    for (id, mut pins) in pins_by_port {
        pins.retain(|p| {
            p.name != "PDR_ON" && p.name != "PC14OSC32_IN" && p.name != "PC15OSC32_OUT"
        });
        pins.sort_by_key(|p| p.number().unwrap_or_default());
        pins.dedup_by_key(|p| p.number().unwrap_or_default());
        ports.push(Port { id, pins });
    }
    ports.sort_by_key(|p| p.id);

    Ok(ports)
}

pub struct PortMacro {
    features: Vec<String>,
    string: String,
}

use core::fmt;

impl fmt::Display for PortMacro {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use lazy_regex::regex_captures;
        if self.features.len() == 1 {
            writeln!(f, r#"#[cfg(feature = "{}")]"#, self.features[0])?;
        } else if self.features.len() < 4 {
            writeln!(
                f,
                "#[cfg(any({}))]",
                self.features
                    .iter()
                    .map(|s| format!(r#"feature = "{s}""#))
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        } else {
            writeln!(
                f,
                "#[cfg(any(\n    {}\n))]",
                self.features
                    .iter()
                    .map(|s| format!(r#"feature = "{s}""#))
                    .collect::<Vec<_>>()
                    .join(",\n    ")
            )?;
        }
        f.write_str("channel_impl! {\n")?;
        let mut strings = self.string.split('\n').collect::<Vec<_>>();
        strings.sort();
        let mut block = String::new();

        let mut prev_periph = String::new();
        let mut prev_altfun = String::new();
        let mut pins = Vec::new();
        for s in &strings {
            let (_whole, periph, altfun, pin) =
                regex_captures!(r#"  (\w+):  <(\w+)> for (.+),"#, s,).unwrap();
            if prev_periph != periph {
                block.push_str(&dump_pins(&prev_altfun, &pins));
                block.push_str(&format!(
                    r##"    }}
}}
pub mod {} {{
    use super::*;
    pin! {{
"##,
                    periph
                ));
                prev_periph = periph.into();
                prev_altfun = altfun.into();
                pins.clear();
                pins.push(pin.into());
            } else if prev_altfun != altfun {
                block.push_str(&dump_pins(&prev_altfun, &pins));

                prev_altfun = altfun.into();
                pins.clear();
                pins.push(pin.into());
            } else {
                pins.push(pin.into());
            }
        }
        block.push_str(&dump_pins(&prev_altfun, &pins));
        block.push_str(
            r##"    }
}
"##,
        );

        f.write_str(&block)?;
        f.write_str("\n}\n")?;
        Ok(())
    }
}

fn dump_pins(altfun: &str, pins: &[String]) -> String {
    let mut block = String::new();
    block.push_str(&format!("        <{altfun}> for [\n"));
    for p in pins {
        block.push_str(&format!("            {p},\n"));
    }
    block.push_str("        ],\n");
    block
}

fn gen_port(feature: &str, map: &mut XMap, port: &Port) -> Result<Vec<String>> {
    let port_upper = port.id;
    let port_lower = port.id.to_ascii_lowercase();
    let mut strings = Vec::new();
    for pin in &port.pins {
        strings.extend(gen_pin(feature, map, port_upper, port_lower, pin)?);
    }

    Ok(strings)
}

fn gen_pin(
    feature: &str,
    map: &mut XMap,
    port_upper: char,
    _port_lower: char,
    pin: &gpio::Pin,
) -> Result<Vec<String>> {
    let nr = pin.number()?;
    let _reset_mode = get_pin_reset_mode(pin)?;
    let af_numbers = get_pin_af_numbers(pin)?;
    let mut strings = Vec::new();

    for (af, func) in af_numbers.into_iter() {
        if let Some(pos) = func.bytes().position(|b| b == b'_') {
            use convert_case::{Case, Casing};
            let per = func[..pos].to_lowercase();
            let pn = (&func[pos + 1..]).to_case(Case::Pascal);
            strings.push(format!("  {per}:  <{pn}> for P{port_upper}{nr}<{af}>,",));
            map.entry(per)
                .or_default()
                .entry(pn)
                .or_default()
                .entry(Pin(port_upper, nr, af))
                .or_default()
                .insert(feature.into());
        } else {
            //    println!("Skipped: {port_lower} {nr} - Unsupported func {func}");
        }
    }
    Ok(strings)
}

fn get_pin_reset_mode(pin: &gpio::Pin) -> Result<Option<&'static str>> {
    // Debug pins default to their debug function (AF0), everything else
    // defaults to floating input or analog.
    let mode = match (pin.port()?, pin.number()?) {
        ('A', 13) | ('A', 14) | ('A', 15) | ('B', 3) | ('B', 4) => Some("super::Debugger"),
        _ => None,
    };
    Ok(mode)
}

fn get_pin_af_numbers(pin: &gpio::Pin) -> Result<Vec<(u8, String)>> {
    let mut numbers = Vec::new();
    for signal in &pin.pin_signals {
        match signal.af() {
            Ok(af) => numbers.push(af),
            Err(_) => {}
        }
    }

    numbers.sort_unstable();
    numbers.dedup();

    Ok(numbers)
}
