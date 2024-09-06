use std::collections::HashMap;
use lorawan::physical_parameters::SpreadingFactor;
use lorawan_device::communicator::ReceivedTransmission;


#[derive(Debug, Default)]
pub struct EWMAContent {
    pub rssi_mean: f32,
    pub rssi_variance: f32,
    
    pub snr_mean: f32,
    pub snr_variance: f32,
    
    pub delay_mean: f32,
    pub delay_variance: f32,
    
    pub counter: u128,
    pub last_timestamp: u128,
}

#[derive(Debug, Default, PartialEq, Eq, Hash)]
pub enum AnomalyLevel {
    #[default]
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

impl AnomalyLevel {
    pub fn is_anomaly(&self) -> bool {
        !matches!(self, AnomalyLevel::None)
    }
}

#[test]
fn t() {
    let rssi_anomaly = true;
        let snr_anomaly = false;
        let delay_anomaly = true;
        let total = rssi_anomaly as u8 + snr_anomaly as u8 + delay_anomaly as u8;
        let level = match total {
            0 => AnomalyLevel::None,
            1 => AnomalyLevel::Low,
            2 => AnomalyLevel::Medium,
            3 => AnomalyLevel::High,
            _ => unreachable!(),
        };
        println!("{:?}", level);
}


impl EWMAContent {
    pub fn update(&mut self, alpha: f32, beta: f32, transmission: &ReceivedTransmission) {
        self.rssi_mean = self.rssi_mean * (1.0 - alpha) + transmission.arrival_stats.rssi * alpha;
        self.snr_mean = self.snr_mean * (1.0 - alpha) + transmission.arrival_stats.snr * alpha;
        self.delay_mean = self.delay_mean * (1.0 - alpha) + (transmission.arrival_stats.time - self.last_timestamp) as f32 * alpha;

        self.rssi_variance = self.rssi_variance * (1.0 - beta) + (transmission.arrival_stats.rssi - self.rssi_mean).powi(2) * beta;
        self.snr_variance = self.snr_variance * (1.0 - beta) + (transmission.arrival_stats.snr - self.snr_mean).powi(2) * beta;
        self.delay_variance = self.delay_variance * (1.0 - beta) + ((transmission.arrival_stats.time - self.last_timestamp) as f32 - self.delay_mean).powi(2) * beta;

        self.last_timestamp = transmission.arrival_stats.time;
        self.counter += 1;
    }
}

#[derive(Debug, Default)]
pub struct AnomalyDetector {
    alpha: f32,
    beta: f32,
    k: f32,
    ewma: HashMap<(SpreadingFactor, u32), EWMAContent>,
}


impl AnomalyDetector {
    pub fn new(alpha: f32, beta: f32, k: f32) -> Self {
        Self {
            alpha,
            beta,
            k,
            ewma: HashMap::new(),
        }
    }

    pub fn anomaly_level(&mut self, transmission: &ReceivedTransmission) -> AnomalyLevel {
        let key = (transmission.transmission.spreading_factor, transmission.transmission.frequency.round() as u32);
        let ewma = self.ewma.entry(key).or_default();

        if ewma.counter < 200 {
            ewma.update(self.alpha, self.beta, transmission);
            AnomalyLevel::None
        } else {
            let anomaly_level = {
                let rssi_anomaly = (transmission.arrival_stats.rssi - ewma.rssi_mean).abs() > self.k * ewma.rssi_variance.sqrt();
                let snr_anomaly = (transmission.arrival_stats.snr - ewma.snr_mean).abs() > self.k * ewma.snr_variance.sqrt();
                let delay_anomaly = (transmission.arrival_stats.time - ewma.last_timestamp) as f32 > self.k * ewma.delay_variance.sqrt();
                let total = rssi_anomaly as u8 + snr_anomaly as u8 + delay_anomaly as u8;
                match total {
                    0 => AnomalyLevel::None,
                    1 => AnomalyLevel::Low,
                    2 => AnomalyLevel::Medium,
                    3 => AnomalyLevel::High,
                    _ => unreachable!(),
                }
            };
            ewma.update(self.alpha, self.beta, transmission);
            anomaly_level
        }
    }
    pub fn start(&mut self) {
        
    }
}


#[cfg(test)]
mod tests {
    use std::fs;

    use lorawan::physical_parameters::LoRaBandwidth;
    use lorawan_device::communicator::{ArrivalStats, Transmission};

    use super::*;

    #[test]
    fn test() {
        let path = "/home/rastafan/Documenti/Dottorato/code/DeLoRaN/DeLoRaN/gateway.csv";
        let mut detector = AnomalyDetector::new(0.2, 0.2, 3.0);
        let content = fs::read_to_string(path).unwrap();
        let rows = content.lines();

        let mut low_map = HashMap::new();
        let mut medium_map = HashMap::new();
        let mut high_map = HashMap::new();

        let mut map = HashMap::new();
        for row in rows.skip(1) {
            let values = row.split(',').collect::<Vec<&str>>();
            let time = values[0].parse::<f32>().unwrap() as u128;
            let frequency = values[3].parse::<f64>().unwrap();
            let sf = SpreadingFactor::new(values[4].parse::<u8>().unwrap());
            let bw = LoRaBandwidth::from(values[5].parse::<f32>().unwrap());
            let rssi = values[6].parse::<f32>().unwrap();
            let snr = values[7].parse::<f32>().unwrap();
            
            let transmission = ReceivedTransmission {
                transmission: Transmission {
                    frequency,
                    bandwidth: bw,
                    spreading_factor: sf,
                    ..Default::default()
                },
                arrival_stats: ArrivalStats {
                    rssi,
                    snr,
                    time,
                },
            };

            let anomaly_level = detector.anomaly_level(&transmission);
            
            match anomaly_level {
                AnomalyLevel::Low => {
                    *low_map.entry((sf, frequency.round() as u32)).or_insert(0_u32) += 1_u32;
                },
                AnomalyLevel::Medium => {
                    *medium_map.entry((sf, frequency.round() as u32)).or_insert(0_u32) += 1_u32;
                },
                AnomalyLevel::High => {
                    println!("High Anomaly Level: {:?}", (sf, frequency.round() as u32));
                    *high_map.entry((sf, frequency.round() as u32)).or_insert(0_u32) += 1_u32;
                },
                _ => {},
            }
            *map.entry(anomaly_level).or_insert(0_u32) += 1;

        }

        println!("Low");

        let mut low = low_map.drain().collect::<Vec<_>>();
        low.sort_by(|((sf1, freq1),count1), ((sf2,freq2), count2)| count2.cmp(count1));
        for (key, value) in low.iter().take(4) {
            println!("{:?} -> {}", key, value);
        }

        println!("Medium");
        let mut medium = medium_map.drain().collect::<Vec<_>>();
        medium.sort_by(|((sf1, freq1),count1), ((sf2,freq2), count2)| count2.cmp(count1));
        for (key, value) in medium.iter().take(4) {
            println!("{:?} -> {}", key, value);
        }

        println!("High");
        let mut high = high_map.drain().collect::<Vec<_>>();
        high.sort_by(|((sf1, freq1),count1), ((sf2,freq2), count2)| count2.cmp(count1));
        for (key, value) in high.iter().take(4) {
            println!("{:?} -> {}", key, value);
        }
    }
}