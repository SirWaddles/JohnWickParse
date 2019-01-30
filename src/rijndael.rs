// Translated to Rust from the reference implementation of rijndael encryption algorithm
// http://www.efgh.com/software/rijndael.htm

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io::Cursor;

pub fn rijndael_setup_encrypt(rk: &mut [u32], key: &[u8]) -> i32 {
    let mut i: i32 = 0;
    let mut temp: u32;
    let mut offset = 0;
    
    let mut key_cursor = Cursor::new(key);
    for e in 0..8 {
        rk[e] = key_cursor.read_u32::<BigEndian>().unwrap();
    }

    loop  {
        temp = rk[7 + offset];
        rk[8 + offset] =
            rk[0 + offset] ^
                TE4[(temp >> 16i32 & 0xffi32 as u32) as usize] &
                    0xff000000u32 as u32 ^
                TE4[(temp >> 8i32 & 0xffi32 as u32) as usize] &
                    0xff0000i32 as u32 ^
                TE4[(temp & 0xffi32 as u32) as usize] &
                    0xff00i32 as u32 ^
                TE4[(temp >> 24i32) as usize] & 0xffi32 as u32 ^
                RCON[i as usize];
        rk[9 + offset] = rk[1 + offset] ^ rk[8 + offset];
        rk[10 + offset] = rk[2 + offset] ^ rk[9 + offset];
        rk[11 + offset] = rk[3 + offset] ^ rk[10 + offset];
        i += 1;
        if i == 7 { 
            return 14;
        }
        temp = rk[11 + offset];
        rk[12 + offset] =
            rk[4 + offset] ^
                TE4[(temp >> 24i32) as usize] &
                    0xff000000u32 as u32 ^
                TE4[(temp >> 16i32 & 0xffi32 as u32) as usize] &
                    0xff0000i32 as u32 ^
                TE4[(temp >> 8i32 & 0xffi32 as u32) as usize] &
                    0xff00i32 as u32 ^
                TE4[(temp & 0xffi32 as u32) as usize] &
                    0xffi32 as u32;
        rk[13 + offset] = rk[5 + offset] ^ rk[12 + offset];
        rk[14 + offset] = rk[6 + offset] ^ rk[13 + offset];
        rk[15 + offset] = rk[7 + offset] ^ rk[14 + offset];
        offset += 8;
    }
}

static TE4: [u32; 256] =
    [0x63636363u32 as u32, 0x7c7c7c7cu32 as u32, 0x77777777u32 as u32,
     0x7b7b7b7bu32 as u32, 0xf2f2f2f2u32 as u32, 0x6b6b6b6bu32 as u32,
     0x6f6f6f6fu32 as u32, 0xc5c5c5c5u32 as u32, 0x30303030u32 as u32,
     0x1010101u32 as u32, 0x67676767u32 as u32, 0x2b2b2b2bu32 as u32,
     0xfefefefeu32 as u32, 0xd7d7d7d7u32 as u32, 0xababababu32 as u32,
     0x76767676u32 as u32, 0xcacacacau32 as u32, 0x82828282u32 as u32,
     0xc9c9c9c9u32 as u32, 0x7d7d7d7du32 as u32, 0xfafafafau32 as u32,
     0x59595959u32 as u32, 0x47474747u32 as u32, 0xf0f0f0f0u32 as u32,
     0xadadadadu32 as u32, 0xd4d4d4d4u32 as u32, 0xa2a2a2a2u32 as u32,
     0xafafafafu32 as u32, 0x9c9c9c9cu32 as u32, 0xa4a4a4a4u32 as u32,
     0x72727272u32 as u32, 0xc0c0c0c0u32 as u32, 0xb7b7b7b7u32 as u32,
     0xfdfdfdfdu32 as u32, 0x93939393u32 as u32, 0x26262626u32 as u32,
     0x36363636u32 as u32, 0x3f3f3f3fu32 as u32, 0xf7f7f7f7u32 as u32,
     0xccccccccu32 as u32, 0x34343434u32 as u32, 0xa5a5a5a5u32 as u32,
     0xe5e5e5e5u32 as u32, 0xf1f1f1f1u32 as u32, 0x71717171u32 as u32,
     0xd8d8d8d8u32 as u32, 0x31313131u32 as u32, 0x15151515u32 as u32,
     0x4040404u32 as u32, 0xc7c7c7c7u32 as u32, 0x23232323u32 as u32,
     0xc3c3c3c3u32 as u32, 0x18181818u32 as u32, 0x96969696u32 as u32,
     0x5050505u32 as u32, 0x9a9a9a9au32 as u32, 0x7070707u32 as u32,
     0x12121212u32 as u32, 0x80808080u32 as u32, 0xe2e2e2e2u32 as u32,
     0xebebebebu32 as u32, 0x27272727u32 as u32, 0xb2b2b2b2u32 as u32,
     0x75757575u32 as u32, 0x9090909u32 as u32, 0x83838383u32 as u32,
     0x2c2c2c2cu32 as u32, 0x1a1a1a1au32 as u32, 0x1b1b1b1bu32 as u32,
     0x6e6e6e6eu32 as u32, 0x5a5a5a5au32 as u32, 0xa0a0a0a0u32 as u32,
     0x52525252u32 as u32, 0x3b3b3b3bu32 as u32, 0xd6d6d6d6u32 as u32,
     0xb3b3b3b3u32 as u32, 0x29292929u32 as u32, 0xe3e3e3e3u32 as u32,
     0x2f2f2f2fu32 as u32, 0x84848484u32 as u32, 0x53535353u32 as u32,
     0xd1d1d1d1u32 as u32, 0u32 as u32, 0xededededu32 as u32,
     0x20202020u32 as u32, 0xfcfcfcfcu32 as u32, 0xb1b1b1b1u32 as u32,
     0x5b5b5b5bu32 as u32, 0x6a6a6a6au32 as u32, 0xcbcbcbcbu32 as u32,
     0xbebebebeu32 as u32, 0x39393939u32 as u32, 0x4a4a4a4au32 as u32,
     0x4c4c4c4cu32 as u32, 0x58585858u32 as u32, 0xcfcfcfcfu32 as u32,
     0xd0d0d0d0u32 as u32, 0xefefefefu32 as u32, 0xaaaaaaaau32 as u32,
     0xfbfbfbfbu32 as u32, 0x43434343u32 as u32, 0x4d4d4d4du32 as u32,
     0x33333333u32 as u32, 0x85858585u32 as u32, 0x45454545u32 as u32,
     0xf9f9f9f9u32 as u32, 0x2020202u32 as u32, 0x7f7f7f7fu32 as u32,
     0x50505050u32 as u32, 0x3c3c3c3cu32 as u32, 0x9f9f9f9fu32 as u32,
     0xa8a8a8a8u32 as u32, 0x51515151u32 as u32, 0xa3a3a3a3u32 as u32,
     0x40404040u32 as u32, 0x8f8f8f8fu32 as u32, 0x92929292u32 as u32,
     0x9d9d9d9du32 as u32, 0x38383838u32 as u32, 0xf5f5f5f5u32 as u32,
     0xbcbcbcbcu32 as u32, 0xb6b6b6b6u32 as u32, 0xdadadadau32 as u32,
     0x21212121u32 as u32, 0x10101010u32 as u32, 0xffffffffu32 as u32,
     0xf3f3f3f3u32 as u32, 0xd2d2d2d2u32 as u32, 0xcdcdcdcdu32 as u32,
     0xc0c0c0cu32 as u32, 0x13131313u32 as u32, 0xececececu32 as u32,
     0x5f5f5f5fu32 as u32, 0x97979797u32 as u32, 0x44444444u32 as u32,
     0x17171717u32 as u32, 0xc4c4c4c4u32 as u32, 0xa7a7a7a7u32 as u32,
     0x7e7e7e7eu32 as u32, 0x3d3d3d3du32 as u32, 0x64646464u32 as u32,
     0x5d5d5d5du32 as u32, 0x19191919u32 as u32, 0x73737373u32 as u32,
     0x60606060u32 as u32, 0x81818181u32 as u32, 0x4f4f4f4fu32 as u32,
     0xdcdcdcdcu32 as u32, 0x22222222u32 as u32, 0x2a2a2a2au32 as u32,
     0x90909090u32 as u32, 0x88888888u32 as u32, 0x46464646u32 as u32,
     0xeeeeeeeeu32 as u32, 0xb8b8b8b8u32 as u32, 0x14141414u32 as u32,
     0xdedededeu32 as u32, 0x5e5e5e5eu32 as u32, 0xb0b0b0bu32 as u32,
     0xdbdbdbdbu32 as u32, 0xe0e0e0e0u32 as u32, 0x32323232u32 as u32,
     0x3a3a3a3au32 as u32, 0xa0a0a0au32 as u32, 0x49494949u32 as u32,
     0x6060606u32 as u32, 0x24242424u32 as u32, 0x5c5c5c5cu32 as u32,
     0xc2c2c2c2u32 as u32, 0xd3d3d3d3u32 as u32, 0xacacacacu32 as u32,
     0x62626262u32 as u32, 0x91919191u32 as u32, 0x95959595u32 as u32,
     0xe4e4e4e4u32 as u32, 0x79797979u32 as u32, 0xe7e7e7e7u32 as u32,
     0xc8c8c8c8u32 as u32, 0x37373737u32 as u32, 0x6d6d6d6du32 as u32,
     0x8d8d8d8du32 as u32, 0xd5d5d5d5u32 as u32, 0x4e4e4e4eu32 as u32,
     0xa9a9a9a9u32 as u32, 0x6c6c6c6cu32 as u32, 0x56565656u32 as u32,
     0xf4f4f4f4u32 as u32, 0xeaeaeaeau32 as u32, 0x65656565u32 as u32,
     0x7a7a7a7au32 as u32, 0xaeaeaeaeu32 as u32, 0x8080808u32 as u32,
     0xbabababau32 as u32, 0x78787878u32 as u32, 0x25252525u32 as u32,
     0x2e2e2e2eu32 as u32, 0x1c1c1c1cu32 as u32, 0xa6a6a6a6u32 as u32,
     0xb4b4b4b4u32 as u32, 0xc6c6c6c6u32 as u32, 0xe8e8e8e8u32 as u32,
     0xddddddddu32 as u32, 0x74747474u32 as u32, 0x1f1f1f1fu32 as u32,
     0x4b4b4b4bu32 as u32, 0xbdbdbdbdu32 as u32, 0x8b8b8b8bu32 as u32,
     0x8a8a8a8au32 as u32, 0x70707070u32 as u32, 0x3e3e3e3eu32 as u32,
     0xb5b5b5b5u32 as u32, 0x66666666u32 as u32, 0x48484848u32 as u32,
     0x3030303u32 as u32, 0xf6f6f6f6u32 as u32, 0xe0e0e0eu32 as u32,
     0x61616161u32 as u32, 0x35353535u32 as u32, 0x57575757u32 as u32,
     0xb9b9b9b9u32 as u32, 0x86868686u32 as u32, 0xc1c1c1c1u32 as u32,
     0x1d1d1d1du32 as u32, 0x9e9e9e9eu32 as u32, 0xe1e1e1e1u32 as u32,
     0xf8f8f8f8u32 as u32, 0x98989898u32 as u32, 0x11111111u32 as u32,
     0x69696969u32 as u32, 0xd9d9d9d9u32 as u32, 0x8e8e8e8eu32 as u32,
     0x94949494u32 as u32, 0x9b9b9b9bu32 as u32, 0x1e1e1e1eu32 as u32,
     0x87878787u32 as u32, 0xe9e9e9e9u32 as u32, 0xcecececeu32 as u32,
     0x55555555u32 as u32, 0x28282828u32 as u32, 0xdfdfdfdfu32 as u32,
     0x8c8c8c8cu32 as u32, 0xa1a1a1a1u32 as u32, 0x89898989u32 as u32,
     0xd0d0d0du32 as u32, 0xbfbfbfbfu32 as u32, 0xe6e6e6e6u32 as u32,
     0x42424242u32 as u32, 0x68686868u32 as u32, 0x41414141u32 as u32,
     0x99999999u32 as u32, 0x2d2d2d2du32 as u32, 0xf0f0f0fu32 as u32,
     0xb0b0b0b0u32 as u32, 0x54545454u32 as u32, 0xbbbbbbbbu32 as u32,
     0x16161616u32 as u32];

static RCON: [u32; 10] =
    [0x1000000i32 as u32, 0x2000000i32 as u32, 0x4000000i32 as u32,
     0x8000000i32 as u32, 0x10000000i32 as u32, 0x20000000i32 as u32,
     0x40000000i32 as u32, 0x80000000u32 as u32, 0x1b000000i32 as u32,
     0x36000000i32 as u32];

pub fn rijndael_setup_decrypt(rk: &mut [u32], key: &[u8]) -> i32 {
    let nrounds: i32 = rijndael_setup_encrypt(rk, key);
    let mut i = 0;
    let mut j = 4 * nrounds;
    let mut temp: u32;

    while i < j {
        temp = rk[i as usize];
        rk[i as usize] = rk[j as usize];
        rk[j as usize] = temp;
        temp = rk[(i + 1i32) as usize];
        rk[(i + 1i32) as usize] = rk[(j + 1i32) as usize];
        rk[(j + 1i32) as usize] = temp;
        temp = rk[(i + 2i32) as usize];
        rk[(i + 2i32) as usize] = rk[(j + 2i32) as usize];
        rk[(j + 2i32) as usize] = temp;
        temp = rk[(i + 3i32) as usize];
        rk[(i + 3i32) as usize] = rk[(j + 3i32) as usize];
        rk[(j + 3i32) as usize] = temp;
        i += 4;
        j -= 4
    }

    let mut offset = 0;

    i = 1;
    while i < nrounds {
        offset += 4;
        rk[0 + offset] =
            TD0[(TE4[(rk[0 + offset] >> 24i32) as usize] &
                     0xffi32 as u32) as usize] ^
                TD1[(TE4[(rk[0 + offset] >> 16i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD2[(TE4[(rk[0 + offset] >> 8i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD3[(TE4[(rk[0 + offset] & 0xffi32 as u32) as
                             usize] & 0xffi32 as u32) as usize];
        rk[1 + offset] =
            TD0[(TE4[(rk[1 + offset] >> 24i32) as usize] &
                     0xffi32 as u32) as usize] ^
                TD1[(TE4[(rk[1 + offset] >> 16i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD2[(TE4[(rk[1 + offset] >> 8i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD3[(TE4[(rk[1 + offset] & 0xffi32 as u32) as
                             usize] & 0xffi32 as u32) as usize];
        rk[2 + offset] =
            TD0[(TE4[(rk[2 + offset] >> 24i32) as usize] &
                     0xffi32 as u32) as usize] ^
                TD1[(TE4[(rk[2 + offset] >> 16i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD2[(TE4[(rk[2 + offset] >> 8i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD3[(TE4[(rk[2 + offset] & 0xffi32 as u32) as
                             usize] & 0xffi32 as u32) as usize];
        rk[3 + offset] =
            TD0[(TE4[(rk[3 + offset] >> 24i32) as usize] &
                     0xffi32 as u32) as usize] ^
                TD1[(TE4[(rk[3 + offset] >> 16i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD2[(TE4[(rk[3 + offset] >> 8i32 &
                              0xffi32 as u32) as usize] &
                         0xffi32 as u32) as usize] ^
                TD3[(TE4[(rk[3 + offset] & 0xffi32 as u32) as
                             usize] & 0xffi32 as u32) as usize];
        i += 1
    }
    return nrounds;
}

static TD3: [u32; 256] =
    [0xf4a75051u32 as u32, 0x4165537eu32 as u32, 0x17a4c31au32 as u32,
     0x275e963au32 as u32, 0xab6bcb3bu32 as u32, 0x9d45f11fu32 as u32,
     0xfa58abacu32 as u32, 0xe303934bu32 as u32, 0x30fa5520u32 as u32,
     0x766df6adu32 as u32, 0xcc769188u32 as u32, 0x24c25f5u32 as u32,
     0xe5d7fc4fu32 as u32, 0x2acbd7c5u32 as u32, 0x35448026u32 as u32,
     0x62a38fb5u32 as u32, 0xb15a49deu32 as u32, 0xba1b6725u32 as u32,
     0xea0e9845u32 as u32, 0xfec0e15du32 as u32, 0x2f7502c3u32 as u32,
     0x4cf01281u32 as u32, 0x4697a38du32 as u32, 0xd3f9c66bu32 as u32,
     0x8f5fe703u32 as u32, 0x929c9515u32 as u32, 0x6d7aebbfu32 as u32,
     0x5259da95u32 as u32, 0xbe832dd4u32 as u32, 0x7421d358u32 as u32,
     0xe0692949u32 as u32, 0xc9c8448eu32 as u32, 0xc2896a75u32 as u32,
     0x8e7978f4u32 as u32, 0x583e6b99u32 as u32, 0xb971dd27u32 as u32,
     0xe14fb6beu32 as u32, 0x88ad17f0u32 as u32, 0x20ac66c9u32 as u32,
     0xce3ab47du32 as u32, 0xdf4a1863u32 as u32, 0x1a3182e5u32 as u32,
     0x51336097u32 as u32, 0x537f4562u32 as u32, 0x6477e0b1u32 as u32,
     0x6bae84bbu32 as u32, 0x81a01cfeu32 as u32, 0x82b94f9u32 as u32,
     0x48685870u32 as u32, 0x45fd198fu32 as u32, 0xde6c8794u32 as u32,
     0x7bf8b752u32 as u32, 0x73d323abu32 as u32, 0x4b02e272u32 as u32,
     0x1f8f57e3u32 as u32, 0x55ab2a66u32 as u32, 0xeb2807b2u32 as u32,
     0xb5c2032fu32 as u32, 0xc57b9a86u32 as u32, 0x3708a5d3u32 as u32,
     0x2887f230u32 as u32, 0xbfa5b223u32 as u32, 0x36aba02u32 as u32,
     0x16825cedu32 as u32, 0xcf1c2b8au32 as u32, 0x79b492a7u32 as u32,
     0x7f2f0f3u32 as u32, 0x69e2a14eu32 as u32, 0xdaf4cd65u32 as u32,
     0x5bed506u32 as u32, 0x34621fd1u32 as u32, 0xa6fe8ac4u32 as u32,
     0x2e539d34u32 as u32, 0xf355a0a2u32 as u32, 0x8ae13205u32 as u32,
     0xf6eb75a4u32 as u32, 0x83ec390bu32 as u32, 0x60efaa40u32 as u32,
     0x719f065eu32 as u32, 0x6e1051bdu32 as u32, 0x218af93eu32 as u32,
     0xdd063d96u32 as u32, 0x3e05aeddu32 as u32, 0xe6bd464du32 as u32,
     0x548db591u32 as u32, 0xc45d0571u32 as u32, 0x6d46f04u32 as u32,
     0x5015ff60u32 as u32, 0x98fb2419u32 as u32, 0xbde997d6u32 as u32,
     0x4043cc89u32 as u32, 0xd99e7767u32 as u32, 0xe842bdb0u32 as u32,
     0x898b8807u32 as u32, 0x195b38e7u32 as u32, 0xc8eedb79u32 as u32,
     0x7c0a47a1u32 as u32, 0x420fe97cu32 as u32, 0x841ec9f8u32 as u32,
     0u32 as u32, 0x80868309u32 as u32, 0x2bed4832u32 as u32,
     0x1170ac1eu32 as u32, 0x5a724e6cu32 as u32, 0xefffbfdu32 as u32,
     0x8538560fu32 as u32, 0xaed51e3du32 as u32, 0x2d392736u32 as u32,
     0xfd9640au32 as u32, 0x5ca62168u32 as u32, 0x5b54d19bu32 as u32,
     0x362e3a24u32 as u32, 0xa67b10cu32 as u32, 0x57e70f93u32 as u32,
     0xee96d2b4u32 as u32, 0x9b919e1bu32 as u32, 0xc0c54f80u32 as u32,
     0xdc20a261u32 as u32, 0x774b695au32 as u32, 0x121a161cu32 as u32,
     0x93ba0ae2u32 as u32, 0xa02ae5c0u32 as u32, 0x22e0433cu32 as u32,
     0x1b171d12u32 as u32, 0x90d0b0eu32 as u32, 0x8bc7adf2u32 as u32,
     0xb6a8b92du32 as u32, 0x1ea9c814u32 as u32, 0xf1198557u32 as u32,
     0x75074cafu32 as u32, 0x99ddbbeeu32 as u32, 0x7f60fda3u32 as u32,
     0x1269ff7u32 as u32, 0x72f5bc5cu32 as u32, 0x663bc544u32 as u32,
     0xfb7e345bu32 as u32, 0x4329768bu32 as u32, 0x23c6dccbu32 as u32,
     0xedfc68b6u32 as u32, 0xe4f163b8u32 as u32, 0x31dccad7u32 as u32,
     0x63851042u32 as u32, 0x97224013u32 as u32, 0xc6112084u32 as u32,
     0x4a247d85u32 as u32, 0xbb3df8d2u32 as u32, 0xf93211aeu32 as u32,
     0x29a16dc7u32 as u32, 0x9e2f4b1du32 as u32, 0xb230f3dcu32 as u32,
     0x8652ec0du32 as u32, 0xc1e3d077u32 as u32, 0xb3166c2bu32 as u32,
     0x70b999a9u32 as u32, 0x9448fa11u32 as u32, 0xe9642247u32 as u32,
     0xfc8cc4a8u32 as u32, 0xf03f1aa0u32 as u32, 0x7d2cd856u32 as u32,
     0x3390ef22u32 as u32, 0x494ec787u32 as u32, 0x38d1c1d9u32 as u32,
     0xcaa2fe8cu32 as u32, 0xd40b3698u32 as u32, 0xf581cfa6u32 as u32,
     0x7ade28a5u32 as u32, 0xb78e26dau32 as u32, 0xadbfa43fu32 as u32,
     0x3a9de42cu32 as u32, 0x78920d50u32 as u32, 0x5fcc9b6au32 as u32,
     0x7e466254u32 as u32, 0x8d13c2f6u32 as u32, 0xd8b8e890u32 as u32,
     0x39f75e2eu32 as u32, 0xc3aff582u32 as u32, 0x5d80be9fu32 as u32,
     0xd0937c69u32 as u32, 0xd52da96fu32 as u32, 0x2512b3cfu32 as u32,
     0xac993bc8u32 as u32, 0x187da710u32 as u32, 0x9c636ee8u32 as u32,
     0x3bbb7bdbu32 as u32, 0x267809cdu32 as u32, 0x5918f46eu32 as u32,
     0x9ab701ecu32 as u32, 0x4f9aa883u32 as u32, 0x956e65e6u32 as u32,
     0xffe67eaau32 as u32, 0xbccf0821u32 as u32, 0x15e8e6efu32 as u32,
     0xe79bd9bau32 as u32, 0x6f36ce4au32 as u32, 0x9f09d4eau32 as u32,
     0xb07cd629u32 as u32, 0xa4b2af31u32 as u32, 0x3f23312au32 as u32,
     0xa59430c6u32 as u32, 0xa266c035u32 as u32, 0x4ebc3774u32 as u32,
     0x82caa6fcu32 as u32, 0x90d0b0e0u32 as u32, 0xa7d81533u32 as u32,
     0x4984af1u32 as u32, 0xecdaf741u32 as u32, 0xcd500e7fu32 as u32,
     0x91f62f17u32 as u32, 0x4dd68d76u32 as u32, 0xefb04d43u32 as u32,
     0xaa4d54ccu32 as u32, 0x9604dfe4u32 as u32, 0xd1b5e39eu32 as u32,
     0x6a881b4cu32 as u32, 0x2c1fb8c1u32 as u32, 0x65517f46u32 as u32,
     0x5eea049du32 as u32, 0x8c355d01u32 as u32, 0x877473fau32 as u32,
     0xb412efbu32 as u32, 0x671d5ab3u32 as u32, 0xdbd25292u32 as u32,
     0x105633e9u32 as u32, 0xd647136du32 as u32, 0xd7618c9au32 as u32,
     0xa10c7a37u32 as u32, 0xf8148e59u32 as u32, 0x133c89ebu32 as u32,
     0xa927eeceu32 as u32, 0x61c935b7u32 as u32, 0x1ce5ede1u32 as u32,
     0x47b13c7au32 as u32, 0xd2df599cu32 as u32, 0xf2733f55u32 as u32,
     0x14ce7918u32 as u32, 0xc737bf73u32 as u32, 0xf7cdea53u32 as u32,
     0xfdaa5b5fu32 as u32, 0x3d6f14dfu32 as u32, 0x44db8678u32 as u32,
     0xaff381cau32 as u32, 0x68c43eb9u32 as u32, 0x24342c38u32 as u32,
     0xa3405fc2u32 as u32, 0x1dc37216u32 as u32, 0xe2250cbcu32 as u32,
     0x3c498b28u32 as u32, 0xd9541ffu32 as u32, 0xa8017139u32 as u32,
     0xcb3de08u32 as u32, 0xb4e49cd8u32 as u32, 0x56c19064u32 as u32,
     0xcb84617bu32 as u32, 0x32b670d5u32 as u32, 0x6c5c7448u32 as u32,
     0xb85742d0u32 as u32];
static TD2: [u32; 256] =
    [0xa75051f4u32 as u32, 0x65537e41u32 as u32, 0xa4c31a17u32 as u32,
     0x5e963a27u32 as u32, 0x6bcb3babu32 as u32, 0x45f11f9du32 as u32,
     0x58abacfau32 as u32, 0x3934be3u32 as u32, 0xfa552030u32 as u32,
     0x6df6ad76u32 as u32, 0x769188ccu32 as u32, 0x4c25f502u32 as u32,
     0xd7fc4fe5u32 as u32, 0xcbd7c52au32 as u32, 0x44802635u32 as u32,
     0xa38fb562u32 as u32, 0x5a49deb1u32 as u32, 0x1b6725bau32 as u32,
     0xe9845eau32 as u32, 0xc0e15dfeu32 as u32, 0x7502c32fu32 as u32,
     0xf012814cu32 as u32, 0x97a38d46u32 as u32, 0xf9c66bd3u32 as u32,
     0x5fe7038fu32 as u32, 0x9c951592u32 as u32, 0x7aebbf6du32 as u32,
     0x59da9552u32 as u32, 0x832dd4beu32 as u32, 0x21d35874u32 as u32,
     0x692949e0u32 as u32, 0xc8448ec9u32 as u32, 0x896a75c2u32 as u32,
     0x7978f48eu32 as u32, 0x3e6b9958u32 as u32, 0x71dd27b9u32 as u32,
     0x4fb6bee1u32 as u32, 0xad17f088u32 as u32, 0xac66c920u32 as u32,
     0x3ab47dceu32 as u32, 0x4a1863dfu32 as u32, 0x3182e51au32 as u32,
     0x33609751u32 as u32, 0x7f456253u32 as u32, 0x77e0b164u32 as u32,
     0xae84bb6bu32 as u32, 0xa01cfe81u32 as u32, 0x2b94f908u32 as u32,
     0x68587048u32 as u32, 0xfd198f45u32 as u32, 0x6c8794deu32 as u32,
     0xf8b7527bu32 as u32, 0xd323ab73u32 as u32, 0x2e2724bu32 as u32,
     0x8f57e31fu32 as u32, 0xab2a6655u32 as u32, 0x2807b2ebu32 as u32,
     0xc2032fb5u32 as u32, 0x7b9a86c5u32 as u32, 0x8a5d337u32 as u32,
     0x87f23028u32 as u32, 0xa5b223bfu32 as u32, 0x6aba0203u32 as u32,
     0x825ced16u32 as u32, 0x1c2b8acfu32 as u32, 0xb492a779u32 as u32,
     0xf2f0f307u32 as u32, 0xe2a14e69u32 as u32, 0xf4cd65dau32 as u32,
     0xbed50605u32 as u32, 0x621fd134u32 as u32, 0xfe8ac4a6u32 as u32,
     0x539d342eu32 as u32, 0x55a0a2f3u32 as u32, 0xe132058au32 as u32,
     0xeb75a4f6u32 as u32, 0xec390b83u32 as u32, 0xefaa4060u32 as u32,
     0x9f065e71u32 as u32, 0x1051bd6eu32 as u32, 0x8af93e21u32 as u32,
     0x63d96ddu32 as u32, 0x5aedd3eu32 as u32, 0xbd464de6u32 as u32,
     0x8db59154u32 as u32, 0x5d0571c4u32 as u32, 0xd46f0406u32 as u32,
     0x15ff6050u32 as u32, 0xfb241998u32 as u32, 0xe997d6bdu32 as u32,
     0x43cc8940u32 as u32, 0x9e7767d9u32 as u32, 0x42bdb0e8u32 as u32,
     0x8b880789u32 as u32, 0x5b38e719u32 as u32, 0xeedb79c8u32 as u32,
     0xa47a17cu32 as u32, 0xfe97c42u32 as u32, 0x1ec9f884u32 as u32,
     0u32 as u32, 0x86830980u32 as u32, 0xed48322bu32 as u32,
     0x70ac1e11u32 as u32, 0x724e6c5au32 as u32, 0xfffbfd0eu32 as u32,
     0x38560f85u32 as u32, 0xd51e3daeu32 as u32, 0x3927362du32 as u32,
     0xd9640a0fu32 as u32, 0xa621685cu32 as u32, 0x54d19b5bu32 as u32,
     0x2e3a2436u32 as u32, 0x67b10c0au32 as u32, 0xe70f9357u32 as u32,
     0x96d2b4eeu32 as u32, 0x919e1b9bu32 as u32, 0xc54f80c0u32 as u32,
     0x20a261dcu32 as u32, 0x4b695a77u32 as u32, 0x1a161c12u32 as u32,
     0xba0ae293u32 as u32, 0x2ae5c0a0u32 as u32, 0xe0433c22u32 as u32,
     0x171d121bu32 as u32, 0xd0b0e09u32 as u32, 0xc7adf28bu32 as u32,
     0xa8b92db6u32 as u32, 0xa9c8141eu32 as u32, 0x198557f1u32 as u32,
     0x74caf75u32 as u32, 0xddbbee99u32 as u32, 0x60fda37fu32 as u32,
     0x269ff701u32 as u32, 0xf5bc5c72u32 as u32, 0x3bc54466u32 as u32,
     0x7e345bfbu32 as u32, 0x29768b43u32 as u32, 0xc6dccb23u32 as u32,
     0xfc68b6edu32 as u32, 0xf163b8e4u32 as u32, 0xdccad731u32 as u32,
     0x85104263u32 as u32, 0x22401397u32 as u32, 0x112084c6u32 as u32,
     0x247d854au32 as u32, 0x3df8d2bbu32 as u32, 0x3211aef9u32 as u32,
     0xa16dc729u32 as u32, 0x2f4b1d9eu32 as u32, 0x30f3dcb2u32 as u32,
     0x52ec0d86u32 as u32, 0xe3d077c1u32 as u32, 0x166c2bb3u32 as u32,
     0xb999a970u32 as u32, 0x48fa1194u32 as u32, 0x642247e9u32 as u32,
     0x8cc4a8fcu32 as u32, 0x3f1aa0f0u32 as u32, 0x2cd8567du32 as u32,
     0x90ef2233u32 as u32, 0x4ec78749u32 as u32, 0xd1c1d938u32 as u32,
     0xa2fe8ccau32 as u32, 0xb3698d4u32 as u32, 0x81cfa6f5u32 as u32,
     0xde28a57au32 as u32, 0x8e26dab7u32 as u32, 0xbfa43fadu32 as u32,
     0x9de42c3au32 as u32, 0x920d5078u32 as u32, 0xcc9b6a5fu32 as u32,
     0x4662547eu32 as u32, 0x13c2f68du32 as u32, 0xb8e890d8u32 as u32,
     0xf75e2e39u32 as u32, 0xaff582c3u32 as u32, 0x80be9f5du32 as u32,
     0x937c69d0u32 as u32, 0x2da96fd5u32 as u32, 0x12b3cf25u32 as u32,
     0x993bc8acu32 as u32, 0x7da71018u32 as u32, 0x636ee89cu32 as u32,
     0xbb7bdb3bu32 as u32, 0x7809cd26u32 as u32, 0x18f46e59u32 as u32,
     0xb701ec9au32 as u32, 0x9aa8834fu32 as u32, 0x6e65e695u32 as u32,
     0xe67eaaffu32 as u32, 0xcf0821bcu32 as u32, 0xe8e6ef15u32 as u32,
     0x9bd9bae7u32 as u32, 0x36ce4a6fu32 as u32, 0x9d4ea9fu32 as u32,
     0x7cd629b0u32 as u32, 0xb2af31a4u32 as u32, 0x23312a3fu32 as u32,
     0x9430c6a5u32 as u32, 0x66c035a2u32 as u32, 0xbc37744eu32 as u32,
     0xcaa6fc82u32 as u32, 0xd0b0e090u32 as u32, 0xd81533a7u32 as u32,
     0x984af104u32 as u32, 0xdaf741ecu32 as u32, 0x500e7fcdu32 as u32,
     0xf62f1791u32 as u32, 0xd68d764du32 as u32, 0xb04d43efu32 as u32,
     0x4d54ccaau32 as u32, 0x4dfe496u32 as u32, 0xb5e39ed1u32 as u32,
     0x881b4c6au32 as u32, 0x1fb8c12cu32 as u32, 0x517f4665u32 as u32,
     0xea049d5eu32 as u32, 0x355d018cu32 as u32, 0x7473fa87u32 as u32,
     0x412efb0bu32 as u32, 0x1d5ab367u32 as u32, 0xd25292dbu32 as u32,
     0x5633e910u32 as u32, 0x47136dd6u32 as u32, 0x618c9ad7u32 as u32,
     0xc7a37a1u32 as u32, 0x148e59f8u32 as u32, 0x3c89eb13u32 as u32,
     0x27eecea9u32 as u32, 0xc935b761u32 as u32, 0xe5ede11cu32 as u32,
     0xb13c7a47u32 as u32, 0xdf599cd2u32 as u32, 0x733f55f2u32 as u32,
     0xce791814u32 as u32, 0x37bf73c7u32 as u32, 0xcdea53f7u32 as u32,
     0xaa5b5ffdu32 as u32, 0x6f14df3du32 as u32, 0xdb867844u32 as u32,
     0xf381caafu32 as u32, 0xc43eb968u32 as u32, 0x342c3824u32 as u32,
     0x405fc2a3u32 as u32, 0xc372161du32 as u32, 0x250cbce2u32 as u32,
     0x498b283cu32 as u32, 0x9541ff0du32 as u32, 0x17139a8u32 as u32,
     0xb3de080cu32 as u32, 0xe49cd8b4u32 as u32, 0xc1906456u32 as u32,
     0x84617bcbu32 as u32, 0xb670d532u32 as u32, 0x5c74486cu32 as u32,
     0x5742d0b8u32 as u32];
static TD1: [u32; 256] =
    [0x5051f4a7u32 as u32, 0x537e4165u32 as u32, 0xc31a17a4u32 as u32,
     0x963a275eu32 as u32, 0xcb3bab6bu32 as u32, 0xf11f9d45u32 as u32,
     0xabacfa58u32 as u32, 0x934be303u32 as u32, 0x552030fau32 as u32,
     0xf6ad766du32 as u32, 0x9188cc76u32 as u32, 0x25f5024cu32 as u32,
     0xfc4fe5d7u32 as u32, 0xd7c52acbu32 as u32, 0x80263544u32 as u32,
     0x8fb562a3u32 as u32, 0x49deb15au32 as u32, 0x6725ba1bu32 as u32,
     0x9845ea0eu32 as u32, 0xe15dfec0u32 as u32, 0x2c32f75u32 as u32,
     0x12814cf0u32 as u32, 0xa38d4697u32 as u32, 0xc66bd3f9u32 as u32,
     0xe7038f5fu32 as u32, 0x9515929cu32 as u32, 0xebbf6d7au32 as u32,
     0xda955259u32 as u32, 0x2dd4be83u32 as u32, 0xd3587421u32 as u32,
     0x2949e069u32 as u32, 0x448ec9c8u32 as u32, 0x6a75c289u32 as u32,
     0x78f48e79u32 as u32, 0x6b99583eu32 as u32, 0xdd27b971u32 as u32,
     0xb6bee14fu32 as u32, 0x17f088adu32 as u32, 0x66c920acu32 as u32,
     0xb47dce3au32 as u32, 0x1863df4au32 as u32, 0x82e51a31u32 as u32,
     0x60975133u32 as u32, 0x4562537fu32 as u32, 0xe0b16477u32 as u32,
     0x84bb6baeu32 as u32, 0x1cfe81a0u32 as u32, 0x94f9082bu32 as u32,
     0x58704868u32 as u32, 0x198f45fdu32 as u32, 0x8794de6cu32 as u32,
     0xb7527bf8u32 as u32, 0x23ab73d3u32 as u32, 0xe2724b02u32 as u32,
     0x57e31f8fu32 as u32, 0x2a6655abu32 as u32, 0x7b2eb28u32 as u32,
     0x32fb5c2u32 as u32, 0x9a86c57bu32 as u32, 0xa5d33708u32 as u32,
     0xf2302887u32 as u32, 0xb223bfa5u32 as u32, 0xba02036au32 as u32,
     0x5ced1682u32 as u32, 0x2b8acf1cu32 as u32, 0x92a779b4u32 as u32,
     0xf0f307f2u32 as u32, 0xa14e69e2u32 as u32, 0xcd65daf4u32 as u32,
     0xd50605beu32 as u32, 0x1fd13462u32 as u32, 0x8ac4a6feu32 as u32,
     0x9d342e53u32 as u32, 0xa0a2f355u32 as u32, 0x32058ae1u32 as u32,
     0x75a4f6ebu32 as u32, 0x390b83ecu32 as u32, 0xaa4060efu32 as u32,
     0x65e719fu32 as u32, 0x51bd6e10u32 as u32, 0xf93e218au32 as u32,
     0x3d96dd06u32 as u32, 0xaedd3e05u32 as u32, 0x464de6bdu32 as u32,
     0xb591548du32 as u32, 0x571c45du32 as u32, 0x6f0406d4u32 as u32,
     0xff605015u32 as u32, 0x241998fbu32 as u32, 0x97d6bde9u32 as u32,
     0xcc894043u32 as u32, 0x7767d99eu32 as u32, 0xbdb0e842u32 as u32,
     0x8807898bu32 as u32, 0x38e7195bu32 as u32, 0xdb79c8eeu32 as u32,
     0x47a17c0au32 as u32, 0xe97c420fu32 as u32, 0xc9f8841eu32 as u32,
     0u32 as u32, 0x83098086u32 as u32, 0x48322bedu32 as u32,
     0xac1e1170u32 as u32, 0x4e6c5a72u32 as u32, 0xfbfd0effu32 as u32,
     0x560f8538u32 as u32, 0x1e3daed5u32 as u32, 0x27362d39u32 as u32,
     0x640a0fd9u32 as u32, 0x21685ca6u32 as u32, 0xd19b5b54u32 as u32,
     0x3a24362eu32 as u32, 0xb10c0a67u32 as u32, 0xf9357e7u32 as u32,
     0xd2b4ee96u32 as u32, 0x9e1b9b91u32 as u32, 0x4f80c0c5u32 as u32,
     0xa261dc20u32 as u32, 0x695a774bu32 as u32, 0x161c121au32 as u32,
     0xae293bau32 as u32, 0xe5c0a02au32 as u32, 0x433c22e0u32 as u32,
     0x1d121b17u32 as u32, 0xb0e090du32 as u32, 0xadf28bc7u32 as u32,
     0xb92db6a8u32 as u32, 0xc8141ea9u32 as u32, 0x8557f119u32 as u32,
     0x4caf7507u32 as u32, 0xbbee99ddu32 as u32, 0xfda37f60u32 as u32,
     0x9ff70126u32 as u32, 0xbc5c72f5u32 as u32, 0xc544663bu32 as u32,
     0x345bfb7eu32 as u32, 0x768b4329u32 as u32, 0xdccb23c6u32 as u32,
     0x68b6edfcu32 as u32, 0x63b8e4f1u32 as u32, 0xcad731dcu32 as u32,
     0x10426385u32 as u32, 0x40139722u32 as u32, 0x2084c611u32 as u32,
     0x7d854a24u32 as u32, 0xf8d2bb3du32 as u32, 0x11aef932u32 as u32,
     0x6dc729a1u32 as u32, 0x4b1d9e2fu32 as u32, 0xf3dcb230u32 as u32,
     0xec0d8652u32 as u32, 0xd077c1e3u32 as u32, 0x6c2bb316u32 as u32,
     0x99a970b9u32 as u32, 0xfa119448u32 as u32, 0x2247e964u32 as u32,
     0xc4a8fc8cu32 as u32, 0x1aa0f03fu32 as u32, 0xd8567d2cu32 as u32,
     0xef223390u32 as u32, 0xc787494eu32 as u32, 0xc1d938d1u32 as u32,
     0xfe8ccaa2u32 as u32, 0x3698d40bu32 as u32, 0xcfa6f581u32 as u32,
     0x28a57adeu32 as u32, 0x26dab78eu32 as u32, 0xa43fadbfu32 as u32,
     0xe42c3a9du32 as u32, 0xd507892u32 as u32, 0x9b6a5fccu32 as u32,
     0x62547e46u32 as u32, 0xc2f68d13u32 as u32, 0xe890d8b8u32 as u32,
     0x5e2e39f7u32 as u32, 0xf582c3afu32 as u32, 0xbe9f5d80u32 as u32,
     0x7c69d093u32 as u32, 0xa96fd52du32 as u32, 0xb3cf2512u32 as u32,
     0x3bc8ac99u32 as u32, 0xa710187du32 as u32, 0x6ee89c63u32 as u32,
     0x7bdb3bbbu32 as u32, 0x9cd2678u32 as u32, 0xf46e5918u32 as u32,
     0x1ec9ab7u32 as u32, 0xa8834f9au32 as u32, 0x65e6956eu32 as u32,
     0x7eaaffe6u32 as u32, 0x821bccfu32 as u32, 0xe6ef15e8u32 as u32,
     0xd9bae79bu32 as u32, 0xce4a6f36u32 as u32, 0xd4ea9f09u32 as u32,
     0xd629b07cu32 as u32, 0xaf31a4b2u32 as u32, 0x312a3f23u32 as u32,
     0x30c6a594u32 as u32, 0xc035a266u32 as u32, 0x37744ebcu32 as u32,
     0xa6fc82cau32 as u32, 0xb0e090d0u32 as u32, 0x1533a7d8u32 as u32,
     0x4af10498u32 as u32, 0xf741ecdau32 as u32, 0xe7fcd50u32 as u32,
     0x2f1791f6u32 as u32, 0x8d764dd6u32 as u32, 0x4d43efb0u32 as u32,
     0x54ccaa4du32 as u32, 0xdfe49604u32 as u32, 0xe39ed1b5u32 as u32,
     0x1b4c6a88u32 as u32, 0xb8c12c1fu32 as u32, 0x7f466551u32 as u32,
     0x49d5eeau32 as u32, 0x5d018c35u32 as u32, 0x73fa8774u32 as u32,
     0x2efb0b41u32 as u32, 0x5ab3671du32 as u32, 0x5292dbd2u32 as u32,
     0x33e91056u32 as u32, 0x136dd647u32 as u32, 0x8c9ad761u32 as u32,
     0x7a37a10cu32 as u32, 0x8e59f814u32 as u32, 0x89eb133cu32 as u32,
     0xeecea927u32 as u32, 0x35b761c9u32 as u32, 0xede11ce5u32 as u32,
     0x3c7a47b1u32 as u32, 0x599cd2dfu32 as u32, 0x3f55f273u32 as u32,
     0x791814ceu32 as u32, 0xbf73c737u32 as u32, 0xea53f7cdu32 as u32,
     0x5b5ffdaau32 as u32, 0x14df3d6fu32 as u32, 0x867844dbu32 as u32,
     0x81caaff3u32 as u32, 0x3eb968c4u32 as u32, 0x2c382434u32 as u32,
     0x5fc2a340u32 as u32, 0x72161dc3u32 as u32, 0xcbce225u32 as u32,
     0x8b283c49u32 as u32, 0x41ff0d95u32 as u32, 0x7139a801u32 as u32,
     0xde080cb3u32 as u32, 0x9cd8b4e4u32 as u32, 0x906456c1u32 as u32,
     0x617bcb84u32 as u32, 0x70d532b6u32 as u32, 0x74486c5cu32 as u32,
     0x42d0b857u32 as u32];
static TD0: [u32; 256] =
    [0x51f4a750u32 as u32, 0x7e416553u32 as u32, 0x1a17a4c3u32 as u32,
     0x3a275e96u32 as u32, 0x3bab6bcbu32 as u32, 0x1f9d45f1u32 as u32,
     0xacfa58abu32 as u32, 0x4be30393u32 as u32, 0x2030fa55u32 as u32,
     0xad766df6u32 as u32, 0x88cc7691u32 as u32, 0xf5024c25u32 as u32,
     0x4fe5d7fcu32 as u32, 0xc52acbd7u32 as u32, 0x26354480u32 as u32,
     0xb562a38fu32 as u32, 0xdeb15a49u32 as u32, 0x25ba1b67u32 as u32,
     0x45ea0e98u32 as u32, 0x5dfec0e1u32 as u32, 0xc32f7502u32 as u32,
     0x814cf012u32 as u32, 0x8d4697a3u32 as u32, 0x6bd3f9c6u32 as u32,
     0x38f5fe7u32 as u32, 0x15929c95u32 as u32, 0xbf6d7aebu32 as u32,
     0x955259dau32 as u32, 0xd4be832du32 as u32, 0x587421d3u32 as u32,
     0x49e06929u32 as u32, 0x8ec9c844u32 as u32, 0x75c2896au32 as u32,
     0xf48e7978u32 as u32, 0x99583e6bu32 as u32, 0x27b971ddu32 as u32,
     0xbee14fb6u32 as u32, 0xf088ad17u32 as u32, 0xc920ac66u32 as u32,
     0x7dce3ab4u32 as u32, 0x63df4a18u32 as u32, 0xe51a3182u32 as u32,
     0x97513360u32 as u32, 0x62537f45u32 as u32, 0xb16477e0u32 as u32,
     0xbb6bae84u32 as u32, 0xfe81a01cu32 as u32, 0xf9082b94u32 as u32,
     0x70486858u32 as u32, 0x8f45fd19u32 as u32, 0x94de6c87u32 as u32,
     0x527bf8b7u32 as u32, 0xab73d323u32 as u32, 0x724b02e2u32 as u32,
     0xe31f8f57u32 as u32, 0x6655ab2au32 as u32, 0xb2eb2807u32 as u32,
     0x2fb5c203u32 as u32, 0x86c57b9au32 as u32, 0xd33708a5u32 as u32,
     0x302887f2u32 as u32, 0x23bfa5b2u32 as u32, 0x2036abau32 as u32,
     0xed16825cu32 as u32, 0x8acf1c2bu32 as u32, 0xa779b492u32 as u32,
     0xf307f2f0u32 as u32, 0x4e69e2a1u32 as u32, 0x65daf4cdu32 as u32,
     0x605bed5u32 as u32, 0xd134621fu32 as u32, 0xc4a6fe8au32 as u32,
     0x342e539du32 as u32, 0xa2f355a0u32 as u32, 0x58ae132u32 as u32,
     0xa4f6eb75u32 as u32, 0xb83ec39u32 as u32, 0x4060efaau32 as u32,
     0x5e719f06u32 as u32, 0xbd6e1051u32 as u32, 0x3e218af9u32 as u32,
     0x96dd063du32 as u32, 0xdd3e05aeu32 as u32, 0x4de6bd46u32 as u32,
     0x91548db5u32 as u32, 0x71c45d05u32 as u32, 0x406d46fu32 as u32,
     0x605015ffu32 as u32, 0x1998fb24u32 as u32, 0xd6bde997u32 as u32,
     0x894043ccu32 as u32, 0x67d99e77u32 as u32, 0xb0e842bdu32 as u32,
     0x7898b88u32 as u32, 0xe7195b38u32 as u32, 0x79c8eedbu32 as u32,
     0xa17c0a47u32 as u32, 0x7c420fe9u32 as u32, 0xf8841ec9u32 as u32,
     0u32 as u32, 0x9808683u32 as u32, 0x322bed48u32 as u32,
     0x1e1170acu32 as u32, 0x6c5a724eu32 as u32, 0xfd0efffbu32 as u32,
     0xf853856u32 as u32, 0x3daed51eu32 as u32, 0x362d3927u32 as u32,
     0xa0fd964u32 as u32, 0x685ca621u32 as u32, 0x9b5b54d1u32 as u32,
     0x24362e3au32 as u32, 0xc0a67b1u32 as u32, 0x9357e70fu32 as u32,
     0xb4ee96d2u32 as u32, 0x1b9b919eu32 as u32, 0x80c0c54fu32 as u32,
     0x61dc20a2u32 as u32, 0x5a774b69u32 as u32, 0x1c121a16u32 as u32,
     0xe293ba0au32 as u32, 0xc0a02ae5u32 as u32, 0x3c22e043u32 as u32,
     0x121b171du32 as u32, 0xe090d0bu32 as u32, 0xf28bc7adu32 as u32,
     0x2db6a8b9u32 as u32, 0x141ea9c8u32 as u32, 0x57f11985u32 as u32,
     0xaf75074cu32 as u32, 0xee99ddbbu32 as u32, 0xa37f60fdu32 as u32,
     0xf701269fu32 as u32, 0x5c72f5bcu32 as u32, 0x44663bc5u32 as u32,
     0x5bfb7e34u32 as u32, 0x8b432976u32 as u32, 0xcb23c6dcu32 as u32,
     0xb6edfc68u32 as u32, 0xb8e4f163u32 as u32, 0xd731dccau32 as u32,
     0x42638510u32 as u32, 0x13972240u32 as u32, 0x84c61120u32 as u32,
     0x854a247du32 as u32, 0xd2bb3df8u32 as u32, 0xaef93211u32 as u32,
     0xc729a16du32 as u32, 0x1d9e2f4bu32 as u32, 0xdcb230f3u32 as u32,
     0xd8652ecu32 as u32, 0x77c1e3d0u32 as u32, 0x2bb3166cu32 as u32,
     0xa970b999u32 as u32, 0x119448fau32 as u32, 0x47e96422u32 as u32,
     0xa8fc8cc4u32 as u32, 0xa0f03f1au32 as u32, 0x567d2cd8u32 as u32,
     0x223390efu32 as u32, 0x87494ec7u32 as u32, 0xd938d1c1u32 as u32,
     0x8ccaa2feu32 as u32, 0x98d40b36u32 as u32, 0xa6f581cfu32 as u32,
     0xa57ade28u32 as u32, 0xdab78e26u32 as u32, 0x3fadbfa4u32 as u32,
     0x2c3a9de4u32 as u32, 0x5078920du32 as u32, 0x6a5fcc9bu32 as u32,
     0x547e4662u32 as u32, 0xf68d13c2u32 as u32, 0x90d8b8e8u32 as u32,
     0x2e39f75eu32 as u32, 0x82c3aff5u32 as u32, 0x9f5d80beu32 as u32,
     0x69d0937cu32 as u32, 0x6fd52da9u32 as u32, 0xcf2512b3u32 as u32,
     0xc8ac993bu32 as u32, 0x10187da7u32 as u32, 0xe89c636eu32 as u32,
     0xdb3bbb7bu32 as u32, 0xcd267809u32 as u32, 0x6e5918f4u32 as u32,
     0xec9ab701u32 as u32, 0x834f9aa8u32 as u32, 0xe6956e65u32 as u32,
     0xaaffe67eu32 as u32, 0x21bccf08u32 as u32, 0xef15e8e6u32 as u32,
     0xbae79bd9u32 as u32, 0x4a6f36ceu32 as u32, 0xea9f09d4u32 as u32,
     0x29b07cd6u32 as u32, 0x31a4b2afu32 as u32, 0x2a3f2331u32 as u32,
     0xc6a59430u32 as u32, 0x35a266c0u32 as u32, 0x744ebc37u32 as u32,
     0xfc82caa6u32 as u32, 0xe090d0b0u32 as u32, 0x33a7d815u32 as u32,
     0xf104984au32 as u32, 0x41ecdaf7u32 as u32, 0x7fcd500eu32 as u32,
     0x1791f62fu32 as u32, 0x764dd68du32 as u32, 0x43efb04du32 as u32,
     0xccaa4d54u32 as u32, 0xe49604dfu32 as u32, 0x9ed1b5e3u32 as u32,
     0x4c6a881bu32 as u32, 0xc12c1fb8u32 as u32, 0x4665517fu32 as u32,
     0x9d5eea04u32 as u32, 0x18c355du32 as u32, 0xfa877473u32 as u32,
     0xfb0b412eu32 as u32, 0xb3671d5au32 as u32, 0x92dbd252u32 as u32,
     0xe9105633u32 as u32, 0x6dd64713u32 as u32, 0x9ad7618cu32 as u32,
     0x37a10c7au32 as u32, 0x59f8148eu32 as u32, 0xeb133c89u32 as u32,
     0xcea927eeu32 as u32, 0xb761c935u32 as u32, 0xe11ce5edu32 as u32,
     0x7a47b13cu32 as u32, 0x9cd2df59u32 as u32, 0x55f2733fu32 as u32,
     0x1814ce79u32 as u32, 0x73c737bfu32 as u32, 0x53f7cdeau32 as u32,
     0x5ffdaa5bu32 as u32, 0xdf3d6f14u32 as u32, 0x7844db86u32 as u32,
     0xcaaff381u32 as u32, 0xb968c43eu32 as u32, 0x3824342cu32 as u32,
     0xc2a3405fu32 as u32, 0x161dc372u32 as u32, 0xbce2250cu32 as u32,
     0x283c498bu32 as u32, 0xff0d9541u32 as u32, 0x39a80171u32 as u32,
     0x80cb3deu32 as u32, 0xd8b4e49cu32 as u32, 0x6456c190u32 as u32,
     0x7bcb8461u32 as u32, 0xd532b670u32 as u32, 0x486c5c74u32 as u32,
     0xd0b85742u32 as u32];

pub fn rijndael_decrypt(rk: &[u32], nrounds: i32, ciphertext: &[u8], plaintext: &mut [u8]) {
    let mut t0: u32;
    let mut t1: u32;
    let mut t2: u32;
    let mut t3: u32;

    let mut cipher_cursor = Cursor::new(ciphertext);
    let mut s0: u32 = cipher_cursor.read_u32::<BigEndian>().unwrap() ^ rk[0];
    let mut s1: u32 = cipher_cursor.read_u32::<BigEndian>().unwrap() ^ rk[1];
    let mut s2: u32 = cipher_cursor.read_u32::<BigEndian>().unwrap() ^ rk[2];
    let mut s3: u32 = cipher_cursor.read_u32::<BigEndian>().unwrap() ^ rk[3];

    t0 =
        TD0[(s0 >> 24i32) as usize] ^
            TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s1 & 0xffi32 as u32) as usize] ^
            rk[4];
    t1 =
        TD0[(s1 >> 24i32) as usize] ^
            TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s2 & 0xffi32 as u32) as usize] ^
            rk[5];
    t2 =
        TD0[(s2 >> 24i32) as usize] ^
            TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s3 & 0xffi32 as u32) as usize] ^
            rk[6];
    t3 =
        TD0[(s3 >> 24i32) as usize] ^
            TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s0 & 0xffi32 as u32) as usize] ^
            rk[7];
    s0 =
        TD0[(t0 >> 24i32) as usize] ^
            TD1[(t3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t1 & 0xffi32 as u32) as usize] ^
            rk[8];
    s1 =
        TD0[(t1 >> 24i32) as usize] ^
            TD1[(t0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t2 & 0xffi32 as u32) as usize] ^
            rk[9];
    s2 =
        TD0[(t2 >> 24i32) as usize] ^
            TD1[(t1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t3 & 0xffi32 as u32) as usize] ^
            rk[10];
    s3 =
        TD0[(t3 >> 24i32) as usize] ^
            TD1[(t2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t0 & 0xffi32 as u32) as usize] ^
            rk[11];
    t0 =
        TD0[(s0 >> 24i32) as usize] ^
            TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s1 & 0xffi32 as u32) as usize] ^
            rk[12];
    t1 =
        TD0[(s1 >> 24i32) as usize] ^
            TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s2 & 0xffi32 as u32) as usize] ^
            rk[13];
    t2 =
        TD0[(s2 >> 24i32) as usize] ^
            TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s3 & 0xffi32 as u32) as usize] ^
            rk[14];
    t3 =
        TD0[(s3 >> 24i32) as usize] ^
            TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s0 & 0xffi32 as u32) as usize] ^
            rk[15];
    s0 =
        TD0[(t0 >> 24i32) as usize] ^
            TD1[(t3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t1 & 0xffi32 as u32) as usize] ^
            rk[16];
    s1 =
        TD0[(t1 >> 24i32) as usize] ^
            TD1[(t0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t2 & 0xffi32 as u32) as usize] ^
            rk[17];
    s2 =
        TD0[(t2 >> 24i32) as usize] ^
            TD1[(t1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t3 & 0xffi32 as u32) as usize] ^
            rk[18];
    s3 =
        TD0[(t3 >> 24i32) as usize] ^
            TD1[(t2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t0 & 0xffi32 as u32) as usize] ^
            rk[19];
    t0 =
        TD0[(s0 >> 24i32) as usize] ^
            TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s1 & 0xffi32 as u32) as usize] ^
            rk[20];
    t1 =
        TD0[(s1 >> 24i32) as usize] ^
            TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s2 & 0xffi32 as u32) as usize] ^
            rk[21];
    t2 =
        TD0[(s2 >> 24i32) as usize] ^
            TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s3 & 0xffi32 as u32) as usize] ^
            rk[22];
    t3 =
        TD0[(s3 >> 24i32) as usize] ^
            TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s0 & 0xffi32 as u32) as usize] ^
            rk[23];
    s0 =
        TD0[(t0 >> 24i32) as usize] ^
            TD1[(t3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t1 & 0xffi32 as u32) as usize] ^
            rk[24];
    s1 =
        TD0[(t1 >> 24i32) as usize] ^
            TD1[(t0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t2 & 0xffi32 as u32) as usize] ^
            rk[25];
    s2 =
        TD0[(t2 >> 24i32) as usize] ^
            TD1[(t1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t3 & 0xffi32 as u32) as usize] ^
            rk[26];
    s3 =
        TD0[(t3 >> 24i32) as usize] ^
            TD1[(t2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t0 & 0xffi32 as u32) as usize] ^
            rk[27];
    t0 =
        TD0[(s0 >> 24i32) as usize] ^
            TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s1 & 0xffi32 as u32) as usize] ^
            rk[28];
    t1 =
        TD0[(s1 >> 24i32) as usize] ^
            TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s2 & 0xffi32 as u32) as usize] ^
            rk[29];
    t2 =
        TD0[(s2 >> 24i32) as usize] ^
            TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s3 & 0xffi32 as u32) as usize] ^
            rk[30];
    t3 =
        TD0[(s3 >> 24i32) as usize] ^
            TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s0 & 0xffi32 as u32) as usize] ^
            rk[31];
    s0 =
        TD0[(t0 >> 24i32) as usize] ^
            TD1[(t3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t1 & 0xffi32 as u32) as usize] ^
            rk[32];
    s1 =
        TD0[(t1 >> 24i32) as usize] ^
            TD1[(t0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t2 & 0xffi32 as u32) as usize] ^
            rk[33];
    s2 =
        TD0[(t2 >> 24i32) as usize] ^
            TD1[(t1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t3 & 0xffi32 as u32) as usize] ^
            rk[34];
    s3 =
        TD0[(t3 >> 24i32) as usize] ^
            TD1[(t2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(t1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(t0 & 0xffi32 as u32) as usize] ^
            rk[35];
    t0 =
        TD0[(s0 >> 24i32) as usize] ^
            TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s1 & 0xffi32 as u32) as usize] ^
            rk[36];
    t1 =
        TD0[(s1 >> 24i32) as usize] ^
            TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s2 & 0xffi32 as u32) as usize] ^
            rk[37];
    t2 =
        TD0[(s2 >> 24i32) as usize] ^
            TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s3 & 0xffi32 as u32) as usize] ^
            rk[38];
    t3 =
        TD0[(s3 >> 24i32) as usize] ^
            TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
            TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
            TD3[(s0 & 0xffi32 as u32) as usize] ^
            rk[39];
    if nrounds > 10i32 {
        s0 =
            TD0[(t0 >> 24i32) as usize] ^
                TD1[(t3 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(t2 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(t1 & 0xffi32 as u32) as usize] ^
                rk[40];
        s1 =
            TD0[(t1 >> 24i32) as usize] ^
                TD1[(t0 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(t3 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(t2 & 0xffi32 as u32) as usize] ^
                rk[41];
        s2 =
            TD0[(t2 >> 24i32) as usize] ^
                TD1[(t1 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(t0 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(t3 & 0xffi32 as u32) as usize] ^
                rk[42];
        s3 =
            TD0[(t3 >> 24i32) as usize] ^
                TD1[(t2 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(t1 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(t0 & 0xffi32 as u32) as usize] ^
                rk[43];
        t0 =
            TD0[(s0 >> 24i32) as usize] ^
                TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(s1 & 0xffi32 as u32) as usize] ^
                rk[44];
        t1 =
            TD0[(s1 >> 24i32) as usize] ^
                TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(s2 & 0xffi32 as u32) as usize] ^
                rk[45];
        t2 =
            TD0[(s2 >> 24i32) as usize] ^
                TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(s3 & 0xffi32 as u32) as usize] ^
                rk[46];
        t3 =
            TD0[(s3 >> 24i32) as usize] ^
                TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
                TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
                TD3[(s0 & 0xffi32 as u32) as usize] ^
                rk[47];
        if nrounds > 12i32 {
            s0 =
                TD0[(t0 >> 24i32) as usize] ^
                    TD1[(t3 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(t2 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(t1 & 0xffi32 as u32) as usize] ^
                    rk[48];
            s1 =
                TD0[(t1 >> 24i32) as usize] ^
                    TD1[(t0 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(t3 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(t2 & 0xffi32 as u32) as usize] ^
                    rk[49];
            s2 =
                TD0[(t2 >> 24i32) as usize] ^
                    TD1[(t1 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(t0 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(t3 & 0xffi32 as u32) as usize] ^
                    rk[50];
            s3 =
                TD0[(t3 >> 24i32) as usize] ^
                    TD1[(t2 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(t1 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(t0 & 0xffi32 as u32) as usize] ^
                    rk[51];
            t0 =
                TD0[(s0 >> 24i32) as usize] ^
                    TD1[(s3 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(s2 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(s1 & 0xffi32 as u32) as usize] ^
                    rk[52];
            t1 =
                TD0[(s1 >> 24i32) as usize] ^
                    TD1[(s0 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(s3 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(s2 & 0xffi32 as u32) as usize] ^
                    rk[53];
            t2 =
                TD0[(s2 >> 24i32) as usize] ^
                    TD1[(s1 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(s0 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(s3 & 0xffi32 as u32) as usize] ^
                    rk[54];
            t3 =
                TD0[(s3 >> 24i32) as usize] ^
                    TD1[(s2 >> 16i32 & 0xffi32 as u32) as usize] ^
                    TD2[(s1 >> 8i32 & 0xffi32 as u32) as usize] ^
                    TD3[(s0 & 0xffi32 as u32) as usize] ^
                    rk[55]
        }
    }
    let rk_offset = (nrounds << 2) as usize;
    s0 =
        TD4[(t0 >> 24i32) as usize] & 0xff000000u32 as u32 ^
            TD4[(t3 >> 16i32 & 0xffi32 as u32) as usize] &
                0xff0000i32 as u32 ^
            TD4[(t2 >> 8i32 & 0xffi32 as u32) as usize] &
                0xff00i32 as u32 ^
            TD4[(t1 & 0xffi32 as u32) as usize] &
                0xffi32 as u32 ^ rk[rk_offset];
    s1 =
        TD4[(t1 >> 24i32) as usize] & 0xff000000u32 as u32 ^
            TD4[(t0 >> 16i32 & 0xffi32 as u32) as usize] &
                0xff0000i32 as u32 ^
            TD4[(t3 >> 8i32 & 0xffi32 as u32) as usize] &
                0xff00i32 as u32 ^
            TD4[(t2 & 0xffi32 as u32) as usize] &
                0xffi32 as u32 ^ rk[rk_offset + 1];
    s2 =
        TD4[(t2 >> 24i32) as usize] & 0xff000000u32 as u32 ^
            TD4[(t1 >> 16i32 & 0xffi32 as u32) as usize] &
                0xff0000i32 as u32 ^
            TD4[(t0 >> 8i32 & 0xffi32 as u32) as usize] &
                0xff00i32 as u32 ^
            TD4[(t3 & 0xffi32 as u32) as usize] &
                0xffi32 as u32 ^ rk[rk_offset + 2];
    s3 =
        TD4[(t3 >> 24i32) as usize] & 0xff000000u32 as u32 ^
            TD4[(t2 >> 16i32 & 0xffi32 as u32) as usize] &
                0xff0000i32 as u32 ^
            TD4[(t1 >> 8i32 & 0xffi32 as u32) as usize] &
                0xff00i32 as u32 ^
            TD4[(t0 & 0xffi32 as u32) as usize] &
                0xffi32 as u32 ^ rk[rk_offset + 3];

    let mut plaintext_cursor = Cursor::new(plaintext);
    plaintext_cursor.write_u32::<BigEndian>(s0).unwrap();
    plaintext_cursor.write_u32::<BigEndian>(s1).unwrap();
    plaintext_cursor.write_u32::<BigEndian>(s2).unwrap();
    plaintext_cursor.write_u32::<BigEndian>(s3).unwrap();
}
static TD4: [u32; 256] =
    [0x52525252u32 as u32, 0x9090909u32 as u32, 0x6a6a6a6au32 as u32,
     0xd5d5d5d5u32 as u32, 0x30303030u32 as u32, 0x36363636u32 as u32,
     0xa5a5a5a5u32 as u32, 0x38383838u32 as u32, 0xbfbfbfbfu32 as u32,
     0x40404040u32 as u32, 0xa3a3a3a3u32 as u32, 0x9e9e9e9eu32 as u32,
     0x81818181u32 as u32, 0xf3f3f3f3u32 as u32, 0xd7d7d7d7u32 as u32,
     0xfbfbfbfbu32 as u32, 0x7c7c7c7cu32 as u32, 0xe3e3e3e3u32 as u32,
     0x39393939u32 as u32, 0x82828282u32 as u32, 0x9b9b9b9bu32 as u32,
     0x2f2f2f2fu32 as u32, 0xffffffffu32 as u32, 0x87878787u32 as u32,
     0x34343434u32 as u32, 0x8e8e8e8eu32 as u32, 0x43434343u32 as u32,
     0x44444444u32 as u32, 0xc4c4c4c4u32 as u32, 0xdedededeu32 as u32,
     0xe9e9e9e9u32 as u32, 0xcbcbcbcbu32 as u32, 0x54545454u32 as u32,
     0x7b7b7b7bu32 as u32, 0x94949494u32 as u32, 0x32323232u32 as u32,
     0xa6a6a6a6u32 as u32, 0xc2c2c2c2u32 as u32, 0x23232323u32 as u32,
     0x3d3d3d3du32 as u32, 0xeeeeeeeeu32 as u32, 0x4c4c4c4cu32 as u32,
     0x95959595u32 as u32, 0xb0b0b0bu32 as u32, 0x42424242u32 as u32,
     0xfafafafau32 as u32, 0xc3c3c3c3u32 as u32, 0x4e4e4e4eu32 as u32,
     0x8080808u32 as u32, 0x2e2e2e2eu32 as u32, 0xa1a1a1a1u32 as u32,
     0x66666666u32 as u32, 0x28282828u32 as u32, 0xd9d9d9d9u32 as u32,
     0x24242424u32 as u32, 0xb2b2b2b2u32 as u32, 0x76767676u32 as u32,
     0x5b5b5b5bu32 as u32, 0xa2a2a2a2u32 as u32, 0x49494949u32 as u32,
     0x6d6d6d6du32 as u32, 0x8b8b8b8bu32 as u32, 0xd1d1d1d1u32 as u32,
     0x25252525u32 as u32, 0x72727272u32 as u32, 0xf8f8f8f8u32 as u32,
     0xf6f6f6f6u32 as u32, 0x64646464u32 as u32, 0x86868686u32 as u32,
     0x68686868u32 as u32, 0x98989898u32 as u32, 0x16161616u32 as u32,
     0xd4d4d4d4u32 as u32, 0xa4a4a4a4u32 as u32, 0x5c5c5c5cu32 as u32,
     0xccccccccu32 as u32, 0x5d5d5d5du32 as u32, 0x65656565u32 as u32,
     0xb6b6b6b6u32 as u32, 0x92929292u32 as u32, 0x6c6c6c6cu32 as u32,
     0x70707070u32 as u32, 0x48484848u32 as u32, 0x50505050u32 as u32,
     0xfdfdfdfdu32 as u32, 0xededededu32 as u32, 0xb9b9b9b9u32 as u32,
     0xdadadadau32 as u32, 0x5e5e5e5eu32 as u32, 0x15151515u32 as u32,
     0x46464646u32 as u32, 0x57575757u32 as u32, 0xa7a7a7a7u32 as u32,
     0x8d8d8d8du32 as u32, 0x9d9d9d9du32 as u32, 0x84848484u32 as u32,
     0x90909090u32 as u32, 0xd8d8d8d8u32 as u32, 0xababababu32 as u32,
     0u32 as u32, 0x8c8c8c8cu32 as u32, 0xbcbcbcbcu32 as u32,
     0xd3d3d3d3u32 as u32, 0xa0a0a0au32 as u32, 0xf7f7f7f7u32 as u32,
     0xe4e4e4e4u32 as u32, 0x58585858u32 as u32, 0x5050505u32 as u32,
     0xb8b8b8b8u32 as u32, 0xb3b3b3b3u32 as u32, 0x45454545u32 as u32,
     0x6060606u32 as u32, 0xd0d0d0d0u32 as u32, 0x2c2c2c2cu32 as u32,
     0x1e1e1e1eu32 as u32, 0x8f8f8f8fu32 as u32, 0xcacacacau32 as u32,
     0x3f3f3f3fu32 as u32, 0xf0f0f0fu32 as u32, 0x2020202u32 as u32,
     0xc1c1c1c1u32 as u32, 0xafafafafu32 as u32, 0xbdbdbdbdu32 as u32,
     0x3030303u32 as u32, 0x1010101u32 as u32, 0x13131313u32 as u32,
     0x8a8a8a8au32 as u32, 0x6b6b6b6bu32 as u32, 0x3a3a3a3au32 as u32,
     0x91919191u32 as u32, 0x11111111u32 as u32, 0x41414141u32 as u32,
     0x4f4f4f4fu32 as u32, 0x67676767u32 as u32, 0xdcdcdcdcu32 as u32,
     0xeaeaeaeau32 as u32, 0x97979797u32 as u32, 0xf2f2f2f2u32 as u32,
     0xcfcfcfcfu32 as u32, 0xcecececeu32 as u32, 0xf0f0f0f0u32 as u32,
     0xb4b4b4b4u32 as u32, 0xe6e6e6e6u32 as u32, 0x73737373u32 as u32,
     0x96969696u32 as u32, 0xacacacacu32 as u32, 0x74747474u32 as u32,
     0x22222222u32 as u32, 0xe7e7e7e7u32 as u32, 0xadadadadu32 as u32,
     0x35353535u32 as u32, 0x85858585u32 as u32, 0xe2e2e2e2u32 as u32,
     0xf9f9f9f9u32 as u32, 0x37373737u32 as u32, 0xe8e8e8e8u32 as u32,
     0x1c1c1c1cu32 as u32, 0x75757575u32 as u32, 0xdfdfdfdfu32 as u32,
     0x6e6e6e6eu32 as u32, 0x47474747u32 as u32, 0xf1f1f1f1u32 as u32,
     0x1a1a1a1au32 as u32, 0x71717171u32 as u32, 0x1d1d1d1du32 as u32,
     0x29292929u32 as u32, 0xc5c5c5c5u32 as u32, 0x89898989u32 as u32,
     0x6f6f6f6fu32 as u32, 0xb7b7b7b7u32 as u32, 0x62626262u32 as u32,
     0xe0e0e0eu32 as u32, 0xaaaaaaaau32 as u32, 0x18181818u32 as u32,
     0xbebebebeu32 as u32, 0x1b1b1b1bu32 as u32, 0xfcfcfcfcu32 as u32,
     0x56565656u32 as u32, 0x3e3e3e3eu32 as u32, 0x4b4b4b4bu32 as u32,
     0xc6c6c6c6u32 as u32, 0xd2d2d2d2u32 as u32, 0x79797979u32 as u32,
     0x20202020u32 as u32, 0x9a9a9a9au32 as u32, 0xdbdbdbdbu32 as u32,
     0xc0c0c0c0u32 as u32, 0xfefefefeu32 as u32, 0x78787878u32 as u32,
     0xcdcdcdcdu32 as u32, 0x5a5a5a5au32 as u32, 0xf4f4f4f4u32 as u32,
     0x1f1f1f1fu32 as u32, 0xddddddddu32 as u32, 0xa8a8a8a8u32 as u32,
     0x33333333u32 as u32, 0x88888888u32 as u32, 0x7070707u32 as u32,
     0xc7c7c7c7u32 as u32, 0x31313131u32 as u32, 0xb1b1b1b1u32 as u32,
     0x12121212u32 as u32, 0x10101010u32 as u32, 0x59595959u32 as u32,
     0x27272727u32 as u32, 0x80808080u32 as u32, 0xececececu32 as u32,
     0x5f5f5f5fu32 as u32, 0x60606060u32 as u32, 0x51515151u32 as u32,
     0x7f7f7f7fu32 as u32, 0xa9a9a9a9u32 as u32, 0x19191919u32 as u32,
     0xb5b5b5b5u32 as u32, 0x4a4a4a4au32 as u32, 0xd0d0d0du32 as u32,
     0x2d2d2d2du32 as u32, 0xe5e5e5e5u32 as u32, 0x7a7a7a7au32 as u32,
     0x9f9f9f9fu32 as u32, 0x93939393u32 as u32, 0xc9c9c9c9u32 as u32,
     0x9c9c9c9cu32 as u32, 0xefefefefu32 as u32, 0xa0a0a0a0u32 as u32,
     0xe0e0e0e0u32 as u32, 0x3b3b3b3bu32 as u32, 0x4d4d4d4du32 as u32,
     0xaeaeaeaeu32 as u32, 0x2a2a2a2au32 as u32, 0xf5f5f5f5u32 as u32,
     0xb0b0b0b0u32 as u32, 0xc8c8c8c8u32 as u32, 0xebebebebu32 as u32,
     0xbbbbbbbbu32 as u32, 0x3c3c3c3cu32 as u32, 0x83838383u32 as u32,
     0x53535353u32 as u32, 0x99999999u32 as u32, 0x61616161u32 as u32,
     0x17171717u32 as u32, 0x2b2b2b2bu32 as u32, 0x4040404u32 as u32,
     0x7e7e7e7eu32 as u32, 0xbabababau32 as u32, 0x77777777u32 as u32,
     0xd6d6d6d6u32 as u32, 0x26262626u32 as u32, 0xe1e1e1e1u32 as u32,
     0x69696969u32 as u32, 0x14141414u32 as u32, 0x63636363u32 as u32,
     0x55555555u32 as u32, 0x21212121u32 as u32, 0xc0c0c0cu32 as u32,
     0x7d7d7d7du32 as u32];

pub fn rijndael_decrypt_buf(ciphertext: &[u8], key: &[u8]) -> Vec<u8> {
    if key.len() != 32 { panic!("Wrong key size"); }
    if ciphertext.len() % 16 != 0 { panic!("Decryption needs to be aligned"); }

    let mut plaintext = vec![0u8; ciphertext.len() as usize];

    let mut rk = [0u32; 60];
    let nrounds = rijndael_setup_decrypt(&mut rk, key);
    for i in 0..(ciphertext.len() / 16) {
        let start = i * 16;
        let end = (i + 1) * 16;
        rijndael_decrypt(&rk, nrounds, &ciphertext[start..end], &mut plaintext[start..end]);
    }

    plaintext
}