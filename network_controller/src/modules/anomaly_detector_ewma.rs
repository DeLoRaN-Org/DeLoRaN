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
    pub anomaly_counter: u128,
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

impl From<u8> for AnomalyLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => AnomalyLevel::None,
            1 => AnomalyLevel::Low,
            2 => AnomalyLevel::Medium,
            3 => AnomalyLevel::High,
            _ => unreachable!(),
        }
    }
}

impl AnomalyLevel {
    pub fn is_anomaly(&self) -> bool {
        matches!(self, AnomalyLevel::Medium | AnomalyLevel::High) 
    }
}

impl EWMAContent {
    pub fn update(&mut self, alpha: f32, beta: f32, transmission: &ReceivedTransmission) {
        self.rssi_variance = self.rssi_variance * (1.0 - beta) + (transmission.arrival_stats.rssi - self.rssi_mean).powi(2) * beta;
        self.snr_variance = self.snr_variance * (1.0 - beta) + (transmission.arrival_stats.snr - self.snr_mean).powi(2) * beta;
        self.delay_variance = self.delay_variance * (1.0 - beta) + ((transmission.arrival_stats.time - self.last_timestamp) as f32 - self.delay_mean).powi(2) * beta;

        self.rssi_mean = self.rssi_mean * (1.0 - alpha) + transmission.arrival_stats.rssi * alpha;
        self.snr_mean = self.snr_mean * (1.0 - alpha) + transmission.arrival_stats.snr * alpha;
        self.delay_mean = self.delay_mean * (1.0 - alpha) + (transmission.arrival_stats.time - self.last_timestamp) as f32 * alpha;

        self.last_timestamp = transmission.arrival_stats.time;
        self.counter += 1;
    }
}

#[derive(Debug, Default)]
pub struct AnomalyDetectorZScore {
    alpha: f32,
    beta: f32,
    k: f32,
    ewma: HashMap<(SpreadingFactor, u32), EWMAContent>,
}


impl AnomalyDetectorZScore {
    pub fn new(alpha: f32, beta: f32, k: f32) -> Self {
        Self {
            alpha,
            beta,
            k,
            ewma: HashMap::new(),
        }
    }

    pub fn update(&mut self, transmission: &ReceivedTransmission) {
        let key = (transmission.transmission.spreading_factor, transmission.transmission.frequency.round() as u32);
        let ewma = self.ewma.entry(key).or_insert(EWMAContent {
            rssi_mean: 0.0,
            rssi_variance: 0.2,
            snr_mean: 0.0,
            snr_variance: 0.2,
            delay_mean: 0.0,
            delay_variance: 0.2,
            counter: 0,
            anomaly_counter: 0,
            last_timestamp: 0,
        });

        ewma.update(self.alpha, self.beta, transmission);
    }

    pub fn update_and_measure_anomaly_level(&mut self, transmission: &ReceivedTransmission) -> AnomalyLevel {
        let key = (transmission.transmission.spreading_factor, transmission.transmission.frequency.round() as u32);
        let ewma = self.ewma.entry(key).or_insert(EWMAContent {
            rssi_mean: 0.0,
            rssi_variance: 0.2,
            snr_mean: 0.0,
            snr_variance: 0.2,
            delay_mean: 0.0,
            delay_variance: 0.2,
            counter: 0,
            anomaly_counter: 0,
            last_timestamp: 0,
        });

        if ewma.counter == 0 {
            ewma.update(self.alpha, self.beta, transmission);
            AnomalyLevel::None
        } else {
            let anomaly_level = {
                let rssi_anomaly = (transmission.arrival_stats.rssi - ewma.rssi_mean).abs() > self.k * ewma.rssi_variance.sqrt();
                let snr_anomaly = (transmission.arrival_stats.snr - ewma.snr_mean).abs() > self.k * ewma.snr_variance.sqrt();
                let delay_anomaly = ((transmission.arrival_stats.time - ewma.last_timestamp) as f32 - ewma.delay_mean).abs() > self.k * ewma.delay_variance.sqrt();
                //let total = snr_anomaly as u8 + delay_anomaly as u8;
                let total = rssi_anomaly as u8 + snr_anomaly as u8 + delay_anomaly as u8;
                AnomalyLevel::from(total)
            };

            if !anomaly_level.is_anomaly() {
                ewma.update(self.alpha, self.beta, transmission);
            }
            anomaly_level
        }
    }
    
    pub fn anomaly_level(&mut self, transmission: &ReceivedTransmission) -> AnomalyLevel {
        let key = (transmission.transmission.spreading_factor, transmission.transmission.frequency.round() as u32);
        let ewma = self.ewma.entry(key).or_insert(EWMAContent {
            rssi_mean: 0.0,
            rssi_variance: 0.2,
            snr_mean: 0.0,
            snr_variance: 0.2,
            delay_mean: 0.0,
            delay_variance: 0.2,
            counter: 0,
            anomaly_counter: 0,
            last_timestamp: 0,
        });

        if ewma.counter == 0 {
            ewma.update(self.alpha, self.beta, transmission);
            AnomalyLevel::None
        } else {
            let rssi_anomaly = (transmission.arrival_stats.rssi - ewma.rssi_mean).abs() > self.k * ewma.rssi_variance.sqrt();
            let snr_anomaly = (transmission.arrival_stats.snr - ewma.snr_mean).abs() > self.k * ewma.snr_variance.sqrt();
            let delay_anomaly = ((transmission.arrival_stats.time - ewma.last_timestamp) as f32 - ewma.delay_mean).abs() > self.k * ewma.delay_variance.sqrt();
            //let total = snr_anomaly as u8 + delay_anomaly as u8;
            let total = rssi_anomaly as u8 + snr_anomaly as u8 + delay_anomaly as u8;
            AnomalyLevel::from(total)
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

    type HelperReturn = (HashMap<AnomalyLevel, u32>, Vec<((SpreadingFactor, u32), u32)>, Vec<((SpreadingFactor, u32), u32)>, Vec<((SpreadingFactor, u32), u32)>, i32, i32, i32, i32);

    fn helper(alpha: f32, beta: f32, k: f32, rows: &[&str]) -> HelperReturn {
        let mut detector = AnomalyDetectorZScore::new(alpha, beta, k);

        //let mut low_map: HashMap<(SpreadingFactor, u32), u32> = HashMap::new();
        //let mut medium_map: HashMap<(SpreadingFactor, u32), u32> = HashMap::new();
        //let mut high_map: HashMap<(SpreadingFactor, u32), u32> = HashMap::new();

        let mut false_positives = 0;
        let mut false_negatives = 0;
        let mut true_positives = 0;
        let mut true_negatives = 0;

        let mut map: HashMap<AnomalyLevel, u32> = HashMap::new();
        for (i, row) in rows.iter().skip(1).enumerate() {
            let values = row.split(',').collect::<Vec<&str>>();
            //println!("{values:?}");

            let time = values[0].parse::<f32>().unwrap() as u128;
            let frequency = values[3].parse::<f64>().unwrap();
            let sf = SpreadingFactor::new(values[4].parse::<u8>().unwrap());
            let bw = LoRaBandwidth::from(values[5].parse::<f32>().unwrap());
            let rssi = values[6].parse::<f32>().unwrap();
            let snr = values[7].parse::<f32>().unwrap();
            let jammer = values[9].parse::<u8>().unwrap() == 1;
            
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

            if i > 4000 {
                //match anomaly_level {
                //    AnomalyLevel::Low => {
                //        *low_map.entry((sf, frequency.round() as u32)).or_insert(0_u32) += 1_u32;
                //    },
                //    AnomalyLevel::Medium => {
                //        *medium_map.entry((sf, frequency.round() as u32)).or_insert(0_u32) += 1_u32;
                //    },
                //    AnomalyLevel::High => {
                //        //println!("High Anomaly Level: {:?}", (sf, frequency.round() as u32));
                //        *high_map.entry((sf, frequency.round() as u32)).or_insert(0_u32) += 1_u32;
                //    },
                //    _ => {},
                //}
                
                let anomaly_level = detector.update_and_measure_anomaly_level(&transmission);
                match (jammer, anomaly_level.is_anomaly()) {
                    (true, true) => true_positives += 1,
                    (true, false) => false_negatives += 1,
                    (false, true) => false_positives += 1,
                    (false, false) => true_negatives += 1,
                }
                *map.entry(anomaly_level).or_insert(0_u32) += 1;
            } else {
                detector.update(&transmission);
            }
        }

        //let mut low = low_map.drain().collect::<Vec<_>>();
        //low.sort_by(|((_, _),count1), ((_,_), count2)| count2.cmp(count1));
        //
        //let mut medium = medium_map.drain().collect::<Vec<_>>();
        //medium.sort_by(|((_, _),count1), ((_,_), count2)| count2.cmp(count1));
        //
        //let mut high = high_map.drain().collect::<Vec<_>>();
        //high.sort_by(|((_, _),count1), ((_,_), count2)| count2.cmp(count1));
        
        (
            map,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            true_positives,
            true_negatives,
            false_positives,
            false_negatives,
        )
    }
    
    #[test]
    fn test() {
        let mut alpha;
        let mut beta;
        let mut k;

        let mut best_tp = 0;
        let mut best_tn = 0;
        let mut best_fp = 0;
        let mut best_fn = 0;
        let mut best_precision = 0.0;
        let mut best_recall = 0.0;
        let mut best_accuracy = 0.0;
        let mut best_f1 = 0.0;

        let mut best_alpha = 0.0;
        let mut best_beta = 0.0;
        let mut best_k = 0.0;

        let path = "/home/rastafan/Documenti/Dottorato/code/DeLoRaN/DeLoRaN/gatewayJammed.csv";
        let content = fs::read_to_string(path).unwrap();
        let rows = content.lines().collect::<Vec<_>>();

        alpha = 0.6;
        while alpha <= 0.9 {
            beta = 0.6;
            while beta <= 0.9 {
                k = 4.0;
                while k < 6.0 {
                    //println!("Alpha: {}, Beta: {}, K: {}", alpha, beta, k);
                    let (_map, _low, _medium, _high, true_positives, true_negatives, false_positives, false_negatives) = helper(alpha, beta, k, &rows);
                    
                    let round_precision = true_positives as f32 / (true_positives + false_positives) as f32;
                    let round_recall = true_positives as f32 / (true_positives + false_negatives) as f32;
                    let f1_score = 2.0 * (round_precision * round_recall) / (round_precision + round_recall);
                    let round_accuracy = (true_positives + true_negatives) as f32 / (true_positives + true_negatives + false_positives + false_negatives) as f32;
                    
                    if f1_score > best_f1 {    
                        best_tp = true_positives;
                        best_tn = true_negatives;
                        best_fp = false_positives;
                        best_fn = false_negatives;
                        best_precision = round_precision;
                        best_recall = round_recall;
                        best_accuracy = round_accuracy;
                        best_f1 = f1_score;

                        best_alpha = alpha;
                        best_beta = beta;
                        best_k = k;
                    } 
                    k += 0.1;
                }
                beta += 0.01;
            }
            alpha += 0.01;
        }

        println!("Best Alpha: {}, Best Beta: {}, Best K: {}", best_alpha, best_beta, best_k);
        println!("Best True Positives: {}, Best True Negatives: {}, Best False Positives: {}, Best False Negatives: {}", best_tp, best_tn, best_fp, best_fn);
        println!("Best Precision: {}, Best Recall: {}, Best Accuracy: {}, Best f1: {}", best_precision, best_recall, best_accuracy, best_f1);
    }


    #[test]
    fn test2() {
        let alpha = 0.82;
        let beta = 0.84;
        let k = 50.0;

        let path = "/home/rastafan/Documenti/Dottorato/code/DeLoRaN/DeLoRaN/network_controller/transformed_gateway_with_jammer_status.csv";
        let content = fs::read_to_string(path).unwrap();
        let rows = content.lines().collect::<Vec<_>>();

        let mut false_positives = 0;
        let mut false_negatives = 0;
        let mut true_positives = 0;
        let mut true_negatives = 0;

        let mut jammer_count = 0;
        let mut not_jammer_count = 0;

        let mut ad = AnomalyDetectorZScore::new(alpha, beta, k);

        for row in rows.iter().skip(1) {
            let values = row.split(',').collect::<Vec<&str>>();
            //println!("{values:?}");

            let time = values[0].parse::<f32>().unwrap() as u128;
            let frequency = values[3].parse::<f64>().unwrap();
            let sf = SpreadingFactor::new(values[4].parse::<u8>().unwrap());
            let bw = LoRaBandwidth::from(values[5].parse::<f32>().unwrap());
            let rssi = values[6].parse::<f32>().unwrap();
            let snr = values[7].parse::<f32>().unwrap();
            let jammer = values[9].parse::<u8>().unwrap() == 1;

            if jammer {
                jammer_count += 1;
            } else {
                not_jammer_count += 1;
            }
            
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

            let anomaly_level = ad.update_and_measure_anomaly_level(&transmission);
            
            match (jammer, anomaly_level.is_anomaly()) {
                (true, true) => true_positives += 1,
                (true, false) => false_negatives += 1,
                (false, true) => false_positives += 1,
                (false, false) => true_negatives += 1,
            }
        }

        println!("True Positives: {}, True Negatives: {}, False Positives: {}, False Negatives: {}", true_positives, true_negatives, false_positives, false_negatives);
        
        let round_precision = true_positives as f32 / (true_positives + false_positives) as f32;
        let round_recall = true_positives as f32 / (true_positives + false_negatives) as f32;
        let f1_score = 2.0 * (round_precision * round_recall) / (round_precision + round_recall);
        let round_accuracy = (true_positives + true_negatives) as f32 / (true_positives + true_negatives + false_positives + false_negatives) as f32;

        println!("Precision: {}, Recall: {}, Accuracy: {}, f1: {}", round_precision, round_recall, round_accuracy, f1_score);
    
    
        println!("Jammer Count: {}, Not Jammer Count: {}", jammer_count, not_jammer_count);
    }
}
