/*
 copyright: (c) 2013-2018 by Blockstack PBC, a public benefit corporation.

 This file is part of Blockstack.

 Blockstack is free software. You may redistribute or modify
 it under the terms of the GNU General Public License as published by
 the Free Software Foundation, either version 3 of the License or
 (at your option) any later version.

 Blockstack is distributed in the hope that it will be useful,
 but WITHOUT ANY WARRANTY, including without the implied warranty of
 MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 GNU General Public License for more details.

 You should have received a copy of the GNU General Public License
 along with Blockstack. If not, see <http://www.gnu.org/licenses/>.
*/
use super::Error;

use sha2::Sha256;
use sha2::Digest;

// TODO: normalize!

const C32_CHARACTERS: &str = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn c32_encode(input_bytes: &[u8]) -> String {
    let c32_chars: &[u8] = C32_CHARACTERS.as_bytes();

    let mut result = vec![];
    let mut carry = 0;
    let mut carry_bits = 0;

    for current_value in input_bytes.iter().rev() {
        let low_bits_to_take = 5 - carry_bits;
        let low_bits = current_value & ((1<<low_bits_to_take) - 1);
        let c32_value = (low_bits << carry_bits) + carry;
        result.push(c32_chars[c32_value as usize]);
        carry_bits = (8 + carry_bits) - 5;
        carry = current_value >> (8 - carry_bits);

        if carry_bits >= 5 {
            let c32_value = carry & ((1<<5) - 1);
            result.push(c32_chars[c32_value as usize]);
            carry_bits = carry_bits - 5;
            carry = carry >> 5;
        }
    }

    if carry_bits > 0 {
        result.push(c32_chars[carry as usize]);
    }

    // remove leading zeros from c32 encoding
    while let Some(v) = result.pop() {
        if v != c32_chars[0] {
            result.push(v);
            break;
        }
    }

    // add leading zeros from input.
    for current_value in input_bytes.iter() {
        if *current_value == 0 {
            result.push(c32_chars[0]);
        } else {
            break;
        }
    }

    let result: Vec<u8> = result.drain(..).rev().collect();
    String::from_utf8(result).unwrap()
}

fn c32_decode(input_str: &str) -> Result<Vec<u8>, Error> {
    let mut result = vec![];
    let mut carry: u16 = 0;
    let mut carry_bits = 0; // can be up to 5

    let iter_c32_digits = input_str.chars().rev()
        .map(|x| { C32_CHARACTERS.find(x) });

    for current_result in iter_c32_digits {
        let current_5bit = current_result.ok_or(Error::InvalidCrockford32)?;
        carry += (current_5bit as u16) << carry_bits;
        carry_bits += 5;

        if carry_bits >= 8 {
            result.push((carry & ((1<<8) - 1)) as u8);
            carry_bits -= 8;
            carry = carry >> 8;
        }
    }

    if carry_bits > 0 {
        result.push(carry as u8);
    }

    // remove leading zeros from Vec<u8> encoding
    while let Some(v) = result.pop() {
        if v != 0 {
            result.push(v);
            break;
        }
    }

    // add leading zeros from input.
    for current_value in input_str.chars() {
        if current_value == '0' {
            result.push(0);
        } else {
            break;
        }
    }

    result.reverse();
    Ok(result)
}

fn double_sha256_checksum(data: &[u8]) -> Vec<u8> {
    let mut sha2 = Sha256::new();
    let mut tmp = [0u8; 32];
    let mut tmp_2 = [0u8; 32];

    sha2.input(data);
    tmp.copy_from_slice(sha2.result().as_slice());

    let mut sha2_2 = Sha256::new();
    sha2_2.input(&tmp);
    tmp_2.copy_from_slice(sha2_2.result().as_slice());

    tmp_2[0..4].to_vec()
}

fn c32_check_encode(version: u8, data: &[u8]) -> Result<String, Error> {
    if version >= 32 {
        return Err(Error::InvalidVersion(version))
    }

    let mut check_data = vec![version];
    check_data.extend_from_slice(data);
    let checksum = double_sha256_checksum(&check_data);

    let mut encoding_data = data.to_vec();
    encoding_data.extend_from_slice(&checksum);

    // working with ascii strings is awful.
    let mut c32_string = c32_encode(&encoding_data).into_bytes();
    let version_char = C32_CHARACTERS.as_bytes()[version as usize];
    c32_string.insert(0, version_char);

    Ok(String::from_utf8(c32_string).unwrap())
}

fn c32_check_decode(check_data: &str) -> Result<(u8, Vec<u8>), Error> {
    if check_data.len() < 2 {
        return Err(Error::InvalidCrockford32)
    }
    let (version, data) = check_data.split_at(1);

    let data_sum_bytes = c32_decode(data)?;
    if data_sum_bytes.len() < 5 {
        return Err(Error::InvalidCrockford32)
    }

    let (data_bytes, expected_sum) = data_sum_bytes.split_at(data_sum_bytes.len() - 4);

    let mut check_data = c32_decode(version)?;
    check_data.extend_from_slice(data_bytes);

    let computed_sum = double_sha256_checksum(&check_data);
    if computed_sum != expected_sum {
        let computed_sum_u32 = 
            (computed_sum[0] as u32) |
            ((computed_sum[1] as u32) << 8) |
            ((computed_sum[2] as u32) << 16) |
            ((computed_sum[3] as u32) << 24);

        let expected_sum_u32 = 
            (expected_sum[0] as u32) |
            ((expected_sum[1] as u32) << 8) |
            ((expected_sum[2] as u32) << 16) |
            ((expected_sum[3] as u32) << 24);

        return Err(Error::BadChecksum(computed_sum_u32, expected_sum_u32));
    }

    let version = check_data[0];
    let data = data_bytes.to_vec();
    Ok((version, data))
}

pub fn c32_address_decode(c32_address_str: &str) -> Result<(u8, Vec<u8>), Error> {
    if c32_address_str.len() <= 5 {
        Err(Error::InvalidCrockford32)
    } else {
        c32_check_decode(&c32_address_str[1..])
    }
}

pub fn c32_address(version: u8, data: &[u8]) -> Result<String, Error> {
    let c32_string = c32_check_encode(version, data)?;
    Ok(format!("S{}", c32_string))
}

#[cfg(test)]
mod test {
    use util::hash::hex_bytes;
    use super::*;

    #[test]
    fn test_addresses() {
        let hex_strs = [
            "a46ff88886c2ef9762d970b4d2c63678835bd39d",
            "0000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000001",
            "1000000000000000000000000000000000000001",
            "1000000000000000000000000000000000000000"
        ];

        let versions = [
            22,
            0,
            31,
            20,
            26,
            21
        ];
  
        let c32_addrs = [
            [
                "SP2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKNRV9EJ7", 
                "SP000000000000000000002Q6VF78",
                "SP00000000000000000005JA84HQ",
                "SP80000000000000000000000000000004R0CMNV", 
                "SP800000000000000000000000000000033H8YKK"
            ],
            [
                "S02J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKPVKG2CE", 
                "S0000000000000000000002AA028H", 
                "S000000000000000000006EKBDDS", 
                "S080000000000000000000000000000007R1QC00", 
                "S080000000000000000000000000000003ENTGCQ"
            ],
            [
                "SZ2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQ9H6DPR", 
                "SZ000000000000000000002ZE1VMN", 
                "SZ00000000000000000005HZ3DVN", 
                "SZ80000000000000000000000000000004XBV6MS", 
                "SZ800000000000000000000000000000007VF5G0"
            ],
            [
                "SM2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQVX8X0G", 
                "SM0000000000000000000062QV6X", 
                "SM00000000000000000005VR75B2", 
                "SM80000000000000000000000000000004WBEWKC", 
                "SM80000000000000000000000000000000JGSYGV"
            ],
            [
                "ST2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKQYAC0RQ", 
                "ST000000000000000000002AMW42H", 
                "ST000000000000000000042DB08Y", 
                "ST80000000000000000000000000000006BYJ4R4", 
                "ST80000000000000000000000000000002YBNPV3"
            ],
            [
                "SN2J6ZY48GV1EZ5V2V5RB9MP66SW86PYKKP6D2ZK9", 
                "SN000000000000000000003YDHWKJ", 
                "SN00000000000000000005341MC8", 
                "SN800000000000000000000000000000066KZWY0", 
                "SN800000000000000000000000000000006H75AK"
            ]
        ];

        for i in 0..hex_strs.len() {
            for j in 0..versions.len() {
                let h = hex_strs[i];
                let v = versions[j];
                let b = hex_bytes(h).unwrap();
                let z = c32_address(v, &b).unwrap();

                assert_eq!(z, c32_addrs[j][i]);

                let (decoded_version, decoded_bytes) = c32_address_decode(&z).unwrap();
                assert_eq!(decoded_version, v);
                assert_eq!(decoded_bytes, b);
            }
        }
    }

    #[test]
    fn test_simple() {
        let hex_strings = &[
              "a46ff88886c2ef9762d970b4d2c63678835bd39d",
              "",
              "0000000000000000000000000000000000000000",
              "0000000000000000000000000000000000000001",
              "1000000000000000000000000000000000000001",
              "1000000000000000000000000000000000000000",
              "01",
              "22",
              "0001",
              "000001",
              "00000001",
              "10",
              "0100",
              "1000",
              "010000",
              "100000",
              "01000000",
              "10000000",
              "0100000000"
          ];
        let c32_strs = [
            "MHQZH246RBQSERPSE2TD5HHPF21NQMWX",
             "",
            "00000000000000000000",
            "00000000000000000001",
            "20000000000000000000000000000001",
            "20000000000000000000000000000000",
            "1",
            "12",
            "01",
            "001",
            "0001",
            "G",
            "80",
            "400",
            "2000",
            "10000",
            "G0000",
            "800000",
            "4000000"
        ];

        let results: Vec<_> = hex_strings.iter().zip(c32_strs.iter())
            .map(|(hex_str, expected)|
                 {
                     let bytes = hex_bytes(hex_str).unwrap();
                     let c32_encoded = c32_encode(&bytes);
                     let decoded_bytes = c32_decode(&c32_encoded).unwrap();
                     let result = (bytes, c32_encoded, decoded_bytes, expected);
                     println!("{:?}", result);
                     result
                 }).collect();
        for (bytes, c32_encoded, decoded_bytes, expected_c32) in results.iter() {
            assert_eq!(bytes, decoded_bytes);
            assert_eq!(c32_encoded, *expected_c32);
        }
    }

    // TODO: test normalization 
}