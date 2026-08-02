use std::{env, fs};

use vibea3::{DecodeOptions, EncodeOptions, Kbin, decode_kbin, encode_kbin};

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "call")]
struct Call {
    #[kbin(attr)]
    model: String,
    player: Player,
}

#[derive(Debug, PartialEq, Kbin)]
#[kbin(node = "player")]
struct Player {
    name: String,
    #[kbin(array)]
    scores: Vec<i32>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let input = args.next().ok_or("usage: interop_fixture INPUT OUTPUT")?;
    let output = args.next().ok_or("usage: interop_fixture INPUT OUTPUT")?;
    if args.next().is_some() {
        return Err("usage: interop_fixture INPUT OUTPUT".into());
    }

    let value = decode_kbin::<Call>(&fs::read(input)?, DecodeOptions::default())?;
    println!("{value:?}");
    fs::write(output, encode_kbin(&value, EncodeOptions::default())?)?;
    Ok(())
}
