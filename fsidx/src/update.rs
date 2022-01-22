use std::collections::BTreeMap;
use std::thread::{self, sleep};
use std::time::Duration;
use super::VolumeInfo;
use nix::sys::stat::stat;

type GroupedVolumes = Vec<Vec<VolumeInfo>>;

pub fn update(volume_info: Vec<VolumeInfo>) {
    let grouped = group_volumes(volume_info);
    let mut handles = vec![];
    for group in grouped {
        let handle = thread::spawn(|| {
            update_volume_group(group);
        });
        handles.push(handle);
    }
    for handle in handles {
        handle.join().expect("join failed");
    }
} 

fn group_volumes(volume_info: Vec<VolumeInfo>) -> GroupedVolumes {
    let mut map = BTreeMap::<i32, Vec::<VolumeInfo>>::new();

    for vi in volume_info {
        let st = stat(&vi.folder);
        if let Ok(f_stat) = st {
            let dev = f_stat.st_dev;
            map
            .entry(dev)
            .or_default()
            .push(vi);
        }
    }

    map
    .values()
    .cloned()
    .collect()
}

fn update_volume_group(group: Vec<VolumeInfo>) {
    for volume_info in group {
        update_volume(volume_info);
    }
}

fn update_volume(volume_info: VolumeInfo) {
    println!("Scanning: {}", volume_info.folder.display());
    sleep(Duration::from_secs(2));
    println!("Finished: {}", volume_info.folder.display());
}
