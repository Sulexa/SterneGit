use std::{env, time::Instant};

use sternegit_reader_sdk::{Folder, ReadInput, Reader, Side, with_files};
use sternegit_unity::Unity;

fn main() {
    let mut args = env::args().skip(1);
    let root = args.next().expect("a Unity project folder");
    let path = args.next().expect("a prefab path inside it");
    let input = ReadInput {
        kind: "prefab".into(),
        path,
        side: Side::After,
        options: Default::default(),
    };
    for run in 1..=3 {
        let started = Instant::now();
        let model = with_files(Folder::new(&root), || Unity.read(input.clone())).expect("the prefab reads");
        let json = serde_json::to_vec(&model).unwrap();
        println!(
            "run {run}: {} objects, {} bytes of JSON, {:?}",
            model.objects.len(),
            json.len(),
            started.elapsed()
        );
    }
}
