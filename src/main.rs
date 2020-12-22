mod bitbuf;

use chrono::{prelude::*, Duration};
use std::fs::File;
use std::io::prelude::*;
use text_io::read;

use bitbuf::BitBuf;

#[derive(Debug, Clone, PartialEq)]
struct BinData {
    stn_update: bool,
    phase: u32,
    intensities: Vec<u8>,
    latitude: f64,
    longitude: f64,
    magnitude: f32,
    depth: f32,
    unix_time: u64,
    id: u32,
    intensity: u32,
    desc: String,
}

impl Default for BinData {
    fn default() -> Self {
        BinData {
            stn_update: false,
            phase: 1,
            intensities: Vec::new(),
            latitude: 30.0,
            longitude: 124.0,
            magnitude: 0.0,
            depth: 0.0,
            unix_time: 0,
            id: 0,
            intensity: 0,
            desc: "SIMULATION".into(),
        }
    }
}

impl BinData {
    fn to_bytes(&self) -> Vec<u8> {
        let mut s = BitBuf::new();

        // Header
        s.write_bit(self.stn_update);
        s.write_int(self.phase as u64, 2);

        // Dummy
        s.write_int(0, 5);
        //s.write_bytes(&[0; 3]); // Size of header is 1 bytes when simulation mode.

        // Body(stations)
        for &stn in &self.intensities {
            s.write_int(stn as u64, 4);
        }
        if self.intensities.len() % 2 != 0 {
            s.write_int(0, 4);
        }

        // Body
        s.write_int((self.latitude * 100.0 - 3000.0) as u64, 10);
        s.write_int((self.longitude * 100.0 - 12400.0) as u64, 10);
        s.write_int((self.magnitude * 10.0) as u64, 7);
        s.write_int((self.depth * 10.0) as u64, 10);
        s.write_int(self.unix_time, 32);
        s.write_int((self.id - 2000000000) as u64, 26);
        s.write_int(self.intensity as u64, 4);
        s.write_int(0, 17); // Dummy for eqk area.

        // Dummy
        s.write_int(0, 4);

        // Description
        s.write_string(&self.desc, 60);

        s.finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Stn {
    latitude: f64,
    longitude: f64,
    mmi: u8,
    mmi_gage: f64,
}

impl Stn {
    fn update(&mut self, decay: f64) {
        if self.mmi_gage > 0.0 {
            self.mmi_gage -= self.mmi_gage * (decay * (0.5 + rand::random::<f64>() * 0.8));
            if self.mmi_gage < 0.0 {
                self.mmi_gage = 0.0;
            }
            self.mmi = self.mmi_gage.round() as u8;
        }
    }
}

struct StnData {
    stations: Vec<Stn>,
}

impl StnData {
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut data = Vec::new();

        let mut num: u32 = 0;
        let mut cnt = 0;
        for b in bytes {
            num <<= 8;
            num |= *b as u32;
            cnt += 8;

            if cnt >= 10 {
                data.push(num >> (cnt - 10));
                cnt -= 10;
                num &= (1 << cnt) - 1;
            }
        }

        let stations = data
            .chunks_exact(2)
            .map(|lat_lon| Stn {
                latitude: 30.0 + lat_lon[0] as f64 / 100.0,
                longitude: 120.0 + lat_lon[1] as f64 / 100.0,
                mmi: 0,
                mmi_gage: 0.0,
            })
            .collect();

        StnData { stations }
    }
}

fn print_flush(s: &str) {
    print!("{}", s);
    std::io::stdout().flush().unwrap();
}

fn lon_to_x(lon: f64) -> f64 {
    (lon - 124.5) * 113.0
}

fn lat_to_y(lat: f64) -> f64 {
    (38.9 - lat) * 138.4
}

fn main() {
    let s_file = File::open("sim_files/stations.s").expect("Prepare sim_files/stations.s file");
    let s = StnData::from_bytes(&s_file.bytes().map(Result::unwrap).collect::<Vec<_>>());
    let mut stations = s.stations;
    println!("Stations: {}", stations.len());

    print_flush("Latitude: ");
    let org_lat: f64 = read!();
    print_flush("Longitude: ");
    let org_lon: f64 = read!();
    println!("Location: {:.2}, {:.2}", org_lat, org_lon);
    let (org_x, org_y) = (lon_to_x(org_lon), lat_to_y(org_lat));

    print_flush("Magnitude: ");
    let magnitude: f32 = read!();
    print_flush("Depth: ");
    let depth: f32 = read!();
    print_flush("Intensity: ");
    let intensity: f64 = read!();
    print_flush("Decay pow: ");
    let decay_pow: f64 = read!();
    print_flush("Decay rate: ");
    let decay_rate: f64 = read!();
    print_flush("Station Decay rate: ");
    let stn_decay_rate: f64 = read!();
    println!(
        "Earthquake: {:.1}M, {:.1}km, MMI-{}",
        magnitude,
        depth,
        intensity.floor()
    );
    println!("Decay Pow: {}, Rate: {}(Stn: {})", decay_pow, decay_rate, stn_decay_rate);

    print_flush("Description: ");
    let desc: String = read!("\n{}\n"); // 이전 엔터 무시하고 입력 완료 엔터까지 받음.
    let desc = "(SIM) ".to_string() + desc.trim();
    let id = 2020123456;

    print_flush("Phase 2 time: ");
    let phase2_time: usize = read!();
    print_flush("Phase 3 time: ");
    let phase3_time: usize = read!();

    let start_time = Utc.ymd(2000, 1, 1).and_hms(0, 0, 0);
    let mut time = start_time;
    let total_time = 300;

    let mut phase = 1;
    let mut mmi_p_gage = intensity / 3.0;
    let mut mmi_s_gage = intensity;

    let mut grid_data = [0u8; 17666 / 2];

    for elapsed in 0..total_time {
        let curr_radius = elapsed as f64 * 3000.0 / 772.5;

        let mut mmi = mmi_s_gage.round() as u8;
        if mmi < 1 {
            mmi = 1;
        } else if mmi > 10 {
            mmi = 10;
        }

        let mut grid_idx = 0;
        let mut i = 38.85;
        'outer: while i > 33.0 {
            let mut j = 124.5;
            while j < 132.05 {
                let sub_x = org_x - lon_to_x(j);
                let sub_y = org_y - lat_to_y(i) + 3.0;

                let distance_sq = sub_x * sub_x + sub_y * sub_y;
                if distance_sq < curr_radius * curr_radius {
                    if grid_idx % 2 == 0 {
                        if (grid_data[grid_idx / 2] >> 4) < mmi {
                            grid_data[grid_idx / 2] = mmi << 4;
                        }
                    } else {
                        if (grid_data[grid_idx / 2] & 0x0f) < mmi {
                            grid_data[grid_idx / 2] |= mmi;
                        }
                    }
                }

                grid_idx += 1;
                if grid_idx >= grid_data.len() * 2 {
                    break 'outer;
                }

                j += 0.05;
            }
            i -= 0.05;
        }

        if mmi_s_gage > 0.0 {
            mmi_s_gage -= mmi_s_gage.powf(decay_pow) * decay_rate;
            if mmi_s_gage < 0.0 {
                mmi_s_gage = 0.0;
            }
        }
    }

    let mut e_file = File::create(format!("sim_files/{}.e", id)).unwrap();
    e_file.write_all(&grid_data).unwrap();
    let mut i_file = File::create(format!("sim_files/{}.i", id)).unwrap();
    i_file.write_all(&grid_data).unwrap();

    let mut mmi_s_gage = intensity;

    for elapsed in 0..total_time {
        time = time + Duration::seconds(1);

        if elapsed >= phase3_time {
            phase = 3;
        } else if elapsed >= phase2_time {
            phase = 2;
        }

        if mmi_p_gage > 0.0 {
            let curr_radius = (elapsed as f64 * 3000.0 / 772.5) * 2.0;
            let next_radius = ((elapsed + 1) as f64 * 3000.0 / 772.5) * 2.0;
            for stn in &mut stations {
                let sub_x = org_x - lon_to_x(stn.longitude);
                let sub_y = org_y - lat_to_y(stn.latitude);

                let distance_sq = sub_x * sub_x + sub_y * sub_y;
                if distance_sq < next_radius * next_radius
                    && distance_sq >= curr_radius * curr_radius
                {
                    if stn.mmi_gage < mmi_p_gage {
                        stn.mmi_gage = mmi_p_gage + 0.4 - rand::random::<f64>() * 0.8;
                        stn.mmi_gage = stn.mmi_gage.max(0.0);
                        stn.mmi = stn.mmi_gage.round() as u8;
                    }
                }
            }

            mmi_p_gage -= mmi_p_gage.powf(decay_pow) * decay_rate;
            if mmi_p_gage < 0.0 {
                mmi_p_gage = 0.0;
            }
        }

        if mmi_s_gage > 0.0 {
            let curr_radius = elapsed as f64 * 3000.0 / 772.5;
            let next_radius = (elapsed + 1) as f64 * 3000.0 / 772.5;
            for stn in &mut stations {
                let sub_x = org_x - lon_to_x(stn.longitude);
                let sub_y = org_y - lat_to_y(stn.latitude);

                let distance_sq = sub_x * sub_x + sub_y * sub_y;
                if distance_sq < next_radius * next_radius
                    && distance_sq >= curr_radius * curr_radius
                {
                    if stn.mmi_gage < mmi_s_gage {
                        stn.mmi_gage = mmi_s_gage + 0.4 - rand::random::<f64>() * 0.8;
                        stn.mmi_gage = stn.mmi_gage.max(0.0);
                        stn.mmi = stn.mmi_gage.round() as u8;
                    }
                }
            }

            mmi_s_gage -= mmi_s_gage.powf(decay_pow) * decay_rate;
            if mmi_s_gage < 0.0 {
                mmi_s_gage = 0.0;
            }
        }

        let b = BinData {
            stn_update: false,
            phase,
            intensities: stations.iter().map(|s| s.mmi).collect(),
            latitude: org_lat,
            longitude: org_lon,
            magnitude,
            depth: if phase > 2 { depth } else { 0.0 },
            unix_time: start_time.timestamp() as u64 - 9 * 3600,
            id,
            intensity: intensity.floor() as u32,
            desc: desc.clone(),
        };
        let bytes = b.to_bytes();

        let time_str = time.format("%Y%m%d%H%M%S");
        let mut b_file = File::create(format!("sim_files/{}.b", time_str)).unwrap();
        b_file.write_all(&bytes).unwrap();

        for stn in &mut stations {
            stn.update(stn_decay_rate);
        }

        println!("{}: {}", time.timestamp(), time);
    }

    let time_str = (start_time + Duration::hours(9) + Duration::seconds(2)).format("%Y%m%d%H%M%S");
    println!("ID: {}, Start time: {}", id, time_str);
}
