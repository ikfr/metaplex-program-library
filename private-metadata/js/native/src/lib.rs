
extern crate bincode;
extern crate curve25519_dalek;
extern crate private_metadata;
#[macro_use]
extern crate serde;
extern crate sha2;
extern crate solana_sdk;
extern crate wasm_bindgen;

use private_metadata::encryption::elgamal::ElGamalKeypair;
use serde::de::{Deserialize, Deserializer, Visitor, SeqAccess, Error};
use serde::ser::{Serialize, SerializeStruct, Serializer, SerializeTuple}; // traits
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signer::keypair::Keypair;
use std::fmt;
use std::marker::PhantomData;
use wasm_bindgen::prelude::*;

trait BigArray<'de>: Sized {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer;
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>;
}

macro_rules! big_array {
    ($($len:expr,)+) => {
        $(
            impl<'de, T> BigArray<'de> for [T; $len]
                where T: Default + Copy + Serialize + Deserialize<'de>
            {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                    where S: Serializer
                {
                    let mut seq = serializer.serialize_tuple(self.len())?;
                    for elem in &self[..] {
                        seq.serialize_element(elem)?;
                    }
                    seq.end()
                }

                fn deserialize<D>(deserializer: D) -> Result<[T; $len], D::Error>
                    where D: Deserializer<'de>
                {
                    struct ArrayVisitor<T> {
                        element: PhantomData<T>,
                    }

                    impl<'de, T> Visitor<'de> for ArrayVisitor<T>
                        where T: Default + Copy + Deserialize<'de>
                    {
                        type Value = [T; $len];

                        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                            formatter.write_str(concat!("an array of length ", $len))
                        }

                        fn visit_seq<A>(self, mut seq: A) -> Result<[T; $len], A::Error>
                            where A: SeqAccess<'de>
                        {
                            let mut arr = [T::default(); $len];
                            for i in 0..$len {
                                arr[i] = seq.next_element()?
                                    .ok_or_else(|| Error::invalid_length(i, &self))?;
                            }
                            Ok(arr)
                        }
                    }

                    let visitor = ArrayVisitor { element: PhantomData };
                    deserializer.deserialize_tuple($len, visitor)
                }
            }
        )+
    }
}

big_array! {
    64,
}

#[derive(Serialize, Deserialize, Debug)]
struct KeypairBytes {
    #[serde(with = "BigArray")]
    bytes: [u8; 64],
}

pub struct JSElGamalKeypair(ElGamalKeypair);

impl Serialize for JSElGamalKeypair {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("JSElGamalKeypair", 2)?;
        s.serialize_field("public", &self.0.public)?;
        s.serialize_field("secret", &self.0.secret)?;
        s.end()
    }
}

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn elgamal_keypair_new(signer: &JsValue, address: &JsValue) -> JsValue {
    log(&format!("Inputs\n\tsigner: {:?}\n\taddress: {:?}", signer, address));

    let signer_bytes: KeypairBytes = signer.into_serde().unwrap();
    let signer = Keypair::from_bytes(&signer_bytes.bytes).unwrap();
    let address: Pubkey = address.into_serde().unwrap();

    log(&format!("Processed Inputs"));

    let kp = ElGamalKeypair::new(&signer, &address).unwrap();
    JsValue::from_serde(&JSElGamalKeypair(kp)).unwrap()
}