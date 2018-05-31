use wallet_crypto::{cbor, address::{ExtendedAddr}};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, App};
use config::{Config};
use storage::{tag, pack};
use blockchain::{Block};

pub struct FindAddress;

impl HasCommand for FindAddress {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "find-addresses";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("retrieve addresses in what have been synced from the network")
            .arg(Arg::with_name("name").help("the network name").index(1).required(true))
            .arg(Arg::with_name("addresses").help("list of addresses to retrieve").multiple(true).required(true).index(2))
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        let name = value_t!(args.value_of("name"), String).unwrap();
        let mut config = Config::default();
        config.network = name;
        let storage = config.get_storage().unwrap();
        let addresses_bytes : Vec<_> = values_t!(args.values_of("addresses"), String)
            .unwrap().iter().map(|s| base58::decode(s).unwrap()).collect();
        let mut addresses : Vec<ExtendedAddr> = vec![];
        for address in addresses_bytes {
            addresses.push(cbor::decode_from_cbor(&address).unwrap());
        }
        let mut iter = storage.iterate_from_epoch(0).unwrap();
        while let Some(blk) = iter.next_block().unwrap() {
            let hdr = blk.get_header();
            let blk_hash = hdr.compute_hash();
            match blk {
                Block::GenesisBlock(_) => {
                    println!("    ignoring {} block", hdr.get_blockdate());
                },
                Block::MainBlock(mblk) => {
                    for txaux in mblk.body.tx.iter() {
                        for txout in &txaux.tx.outputs {
                            if let Some(_) = addresses.iter().find(|a| *a == &txout.address) {
                                println!("found address: {} in block {} at {}",
                                    base58::encode(&cbor::encode_to_cbor(&txout.address).unwrap()),
                                    blk_hash,
                                    hdr.get_blockdate()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
