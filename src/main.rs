extern crate serde_yaml;
extern crate discord;

use std::fs::File;

use std::io::Read;
use std::collections::BTreeMap;

fn main() {
    let mut f = File::open("config.yaml").unwrap();
    let mut fstr = String::new();
    let _ = f.read_to_string(&mut fstr);

    let config: BTreeMap<String, String> = serde_yaml::from_str(&mut fstr).unwrap();

}
