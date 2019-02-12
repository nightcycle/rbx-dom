use std::io::{Read, Write};

use rbx_tree::RbxValue;

use crate::{
    deserializer::{DecodeError, EventIterator},
    serializer::{EncodeError, XmlWriteEvent, XmlEventWriter},
};

static TAG_NAMES: [&str; 12] = ["X", "Y", "Z", "R00", "R01", "R02", "R10", "R11", "R12", "R20", "R21", "R22"];

pub fn serialize_cframe<W: Write>(
    writer: &mut XmlEventWriter<W>,
    name: &str,
    value: [f32; 12],
) -> Result<(), EncodeError> {
    writer.write(XmlWriteEvent::start_element("CoordinateFrame").attr("name", name))?;
    writer.write_tag_array(&value, &TAG_NAMES)?;
    writer.write(XmlWriteEvent::end_element())?;

    Ok(())
}

pub fn deserialize_cframe<R: Read>(reader: &mut EventIterator<R>) -> Result<RbxValue, DecodeError> {
    reader.expect_start_with_name("CoordinateFrame")?;

    let mut components = [0.0; 12];

    for index in 0..12 {
        let tag_name = TAG_NAMES[index];
        components[index] = reader.read_tag_contents(tag_name)?.parse()?;
    }

    reader.expect_end_with_name("CoordinateFrame")?;

    Ok(RbxValue::CFrame {
        value: components,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn round_trip() {
        let _ = env_logger::try_init();

        let test_input: [f32; 12] = [
            123.0, 456.0, 789.0,
            987.0, 654.0, 432.0,
            210.0, 0.0, -12345.0,
            765.0, 234.0, 123123.0,
        ];
        let mut buffer = Vec::new();

        let mut writer = XmlEventWriter::from_output(&mut buffer);
        serialize_cframe(&mut writer, "foo", test_input).unwrap();

        println!("{}", std::str::from_utf8(&buffer).unwrap());

        let mut reader = EventIterator::from_source(buffer.as_slice());
        reader.next().unwrap().unwrap(); // Eat StartDocument event
        let value = deserialize_cframe(&mut reader).unwrap();

        assert_eq!(value, RbxValue::CFrame {
            value: test_input,
        });
    }
}