use std::{error::Error, fs, path::Path};

use indexmap::IndexMap;

#[derive(Debug)]
pub(crate) enum Value {
    Object(IndexMap<String, Value>),
    String(String),
    Int(i32),
}

fn read_vdf_byte(input: &mut &[u8]) -> Result<u8, Box<dyn Error>> {
    let (&byte, rest) = input.split_first().ok_or("Unexpected EOF")?;

    *input = rest;

    Ok(byte)
}

fn read_vdf_string(input: &mut &[u8]) -> Result<String, Box<dyn Error>> {
    let end = input
        .iter()
        .position(|&byte| byte == 0x00)
        .ok_or("Missing null terminator")?;

    let string = std::str::from_utf8(&input[..end])?.to_owned();

    *input = &input[end + 1..];

    Ok(string)
}

fn read_vdf_i32(input: &mut &[u8]) -> Result<i32, Box<dyn Error>> {
    if input.len() < 4 {
        return Err("Unexpected EOF".into());
    }

    let (bytes, rest) = input.split_at(4);

    let value = i32::from_le_bytes(bytes.try_into()?);

    *input = rest;

    Ok(value)
}

fn read_vdf_object(input: &mut &[u8]) -> Result<IndexMap<String, Value>, Box<dyn Error>> {
    let mut map = IndexMap::new();

    loop {
        let next = read_vdf_byte(input)?;

        if next == 0x08 {
            break;
        }

        let key = read_vdf_string(input)?;

        let value = match next {
            0x00 => Value::Object(read_vdf_object(input)?),
            0x01 => Value::String(read_vdf_string(input)?),
            0x02 => Value::Int(read_vdf_i32(input)?),
            _ => return Err(format!("Unknown type: {next:#04x}").into()),
        };

        map.insert(key, value);
    }

    Ok(map)
}

pub(crate) fn read_vdf_shortcuts(path: &Path) -> Result<IndexMap<String, Value>, Box<dyn Error>> {
    let raw_vdf_bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(IndexMap::new());
        }
        Err(error) => return Err(error.into()),
    };

    let mut input = raw_vdf_bytes.as_slice();

    let root = read_vdf_byte(&mut input)?;

    if root != 0x00 {
        return Err("Expected root object".into());
    }

    let key = read_vdf_string(&mut input)?;

    if key != "shortcuts" {
        return Err("Expected shortcuts root".into());
    }

    read_vdf_object(&mut input)
}

fn write_vdf_string(output: &mut Vec<u8>, value: &str) {
    output.extend_from_slice(value.as_bytes());
    output.push(0x00);
}

fn write_vdf_object(output: &mut Vec<u8>, entries: &IndexMap<String, Value>) {
    for (key, value) in entries {
        match value {
            Value::Object(fields) => {
                output.push(0x00);
                write_vdf_string(output, key);
                write_vdf_object(output, fields);
            }
            Value::String(value) => {
                output.push(0x01);
                write_vdf_string(output, key);
                write_vdf_string(output, value);
            }
            Value::Int(value) => {
                output.push(0x02);
                write_vdf_string(output, key);
                output.extend_from_slice(&value.to_le_bytes());
            }
        }
    }

    output.push(0x08);
}

pub(crate) fn write_vdf_shortcuts(
    path: &Path,
    shortcuts: &IndexMap<String, Value>,
) -> Result<(), Box<dyn Error>> {
    let mut output = vec![0x00];
    write_vdf_string(&mut output, "shortcuts");
    write_vdf_object(&mut output, shortcuts);
    output.push(0x08);

    fs::write(path, output)?;
    Ok(())
}
