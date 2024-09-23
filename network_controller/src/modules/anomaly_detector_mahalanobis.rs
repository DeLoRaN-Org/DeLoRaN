use std::collections::HashMap;

use lorawan::physical_parameters::SpreadingFactor;
use nalgebra::{DMatrix, DVector};


#[derive(Debug)]
pub struct AnomalyDetectorMahalnobis {
    threshold: f64,
    num_features: usize,
    mahalnobis: HashMap<(SpreadingFactor, String), Mahalnobis>,
}

impl AnomalyDetectorMahalnobis {
    pub fn new(threshold: f64, num_features: usize) -> Self {
        AnomalyDetectorMahalnobis {
            threshold,
            num_features,
            mahalnobis: HashMap::new(),
        }
    }

    pub fn update(&mut self, sf: SpreadingFactor, frequency: String, row: &DVector<f64>) {
        let key = (sf, frequency.clone());
        let detector = self.mahalnobis.entry(key).or_insert(Mahalnobis::new(sf, frequency, self.threshold, self.num_features));
        detector.update(row);
    }

    pub fn update_rows(&mut self, sf: SpreadingFactor, frequency: String, data: &[DVector<f64>]) {
        let key = (sf, frequency.clone());
        let detector = self.mahalnobis.entry(key).or_insert(Mahalnobis::new(sf, frequency, self.threshold, self.num_features));
        detector.update_rows(data);
    }

    pub fn is_anomaly(&mut self, sf: SpreadingFactor, frequency: String, row: &DVector<f64>) -> (bool, f64) {
        let key = (sf, frequency.clone());
        let detector = self.mahalnobis.entry(key).or_insert(Mahalnobis::new(sf, frequency, self.threshold, self.num_features));
        detector.is_anomaly(row)
    }

    pub fn mean(&self, sf: SpreadingFactor, fq: String) -> &DVector<f64> {
        &self.mahalnobis[&(sf, fq)].mean
    }
    
    //pub fn variance(&self, sf: SpreadingFactor, fq: String) -> &DVector<f64> {
    //    &self.mahalnobis[&(sf, fq)].variance
    //}
    
    pub fn get_mahalanobi(&self, sf: SpreadingFactor, fq: String) -> &Mahalnobis {
        &self.mahalnobis[&(sf, fq)]
    }
    
    pub fn covariance_matrix(&self, sf: SpreadingFactor, fq: String) -> &DMatrix<f64> {
        &self.mahalnobis[&(sf, fq)].covariance_matrix
    }
}

#[derive(Debug)]
pub struct Mahalnobis {
    sf: SpreadingFactor,
    freq: String,
    //id: String,
    mean: DVector<f64>,
    //m2: DVector<f64>,
    //variance: DVector<f64>,
    covariance_matrix: DMatrix<f64>,
    threshold: f64,
    counter: usize,
    last_timestamp: u128,
}

impl Mahalnobis {
    pub fn new(sf: SpreadingFactor, fq: String, threshold: f64, num_features: usize) -> Self {
        Mahalnobis {
            //id: format!("{:?}", Instant::now()),
            //m2: DVector::from_element(num_features, 0.0),
            //variance: DVector::from_element(num_features, 1.0)
            sf,
            freq: fq,
            mean: DVector::from_element(num_features, 0.0),
            covariance_matrix: DMatrix::from_element(num_features, num_features, 1e-5),
            threshold,
            counter: 0,
            last_timestamp: 0,
        }
    }

    pub fn update(&mut self, row: &DVector<f64>) {
        //let tmst = row[2] as u128;
        //
        //if tmst < self.last_timestamp {
        //    self.last_timestamp = tmst;
        //    return;
        //}
        //
        //let mut row_c = row.clone();
        //row_c[2] = (row_c[2] - self.last_timestamp as f64).ln_1p();
        
        //println!("{} -- {} -- {}", self.mean, self.covariance_matrix, row_c);
        //let normalized_row = self.normalize_row(&row_c);
        
        self.update_mean(row);
        self.update_covariance(row);
        
        self.counter += 1;
        //self.last_timestamp = tmst;

        //if row_c[2] > 3000.0 {
        //    println!("{} -- {} -- {}", self.mean, self.covariance_matrix, row_c);
        //}
        
    }

    //pub fn normalize_row(&self, row: &DVector<f64>) -> DVector<f64> {
    //    let mut std_dev = self.variance.clone();
    //    std_dev.apply(|arg0: &mut f64| { *arg0 = f64::sqrt(*arg0); });
    //    (row - &self.mean).component_div(&std_dev)
    //}

    pub fn update_rows(&mut self, data: &[DVector<f64>]) {
        for row in data {
            self.update(row);
        }
    }
    
    pub fn update_mean(&mut self, row: &DVector<f64>) {
        let n = self.counter as f64;

        if n <= 2.0 {
            self.mean = row.clone();
            return;
        }
        let old_mean = self.mean.clone();
        self.mean = &old_mean + (row - &old_mean) / (n + 1.0);

        //let delta = row - &self.mean;
        //self.mean += &delta / n;
        //let delta2 = row - &self.mean;
        //self.m2 += delta.component_mul(&delta2);
        //if self.counter > 1 {
        //    self.variance = &self.m2 / n;
        //}
    }

    pub fn update_covariance(&mut self, row: &DVector<f64>) {
        let n = self.counter as f64;
        if n <= 2.0 {
            return;
        }

        let left_product =  ((n - 1.0)/ (n)) * &self.covariance_matrix;
        let right_product = (1.0 / n) * (row - &self.mean) * (row - &self.mean).transpose();

        //let diff = row - &self.mean;
        //let outer_product = &diff * diff.transpose() * (n + 1.0);
        self.covariance_matrix = &left_product + &right_product;
        if self.covariance_matrix.data.as_slice().iter().any(|arg0: &f64| f64::is_nan(*arg0)) {
            println!("{:?}", row);
            println!("{:?}", self.mean);
            println!("{:?}", left_product);
            println!("{:?}", right_product);
            
            println!("{:?}", &self.counter);
            println!("{:?}", &self.covariance_matrix);
            panic!("Covariance matrix contains NaN values");
        }
    }

    pub fn mahalanobis_distance(&mut self, row: &DVector<f64>) -> f64 {
        //let normalized_row = self.normalize_row(row);
        
        //let regularization = 1e-4;  // A small regularization constant
        //let reg_cov_matrix = &self.covariance_matrix + DMatrix::<f64>::identity(self.covariance_matrix.nrows(), self.covariance_matrix.ncols()) * regularization;
        //println!("{} {} - {}", &self.sf, &self.freq, &self.covariance_matrix);
        let inv_cov = match self.covariance_matrix.clone().try_inverse() {
            Some(inv) => inv,
            None => {
                println!("{:?}", &self.covariance_matrix);
                println!("{:?}", &self.covariance_matrix.determinant());
                println!("{:?} - {} - {}", &self.counter, &self.sf, &self.freq);
                //panic!("Regularized covariance matrix should be invertible");
                let regularization = 1e-4;  // A small regularization constant
                let reg_cov_matrix = &self.covariance_matrix + DMatrix::<f64>::identity(self.covariance_matrix.nrows(), self.covariance_matrix.ncols()) * regularization;
                reg_cov_matrix.try_inverse().expect("Regularized covariance matrix should be invertible")
            }
        };
        //LOGGER2.write_sync(&format!("{:?}", &self.covariance_matrix));
        //LOGGER2.write_sync(&format!("{:?}", inv_cov));
        let diff = row - &self.mean;
        (&diff.transpose() * inv_cov * &diff)[(0,0)].sqrt()
    }

    pub fn is_anomaly(&mut self, row: &DVector<f64>) -> (bool, f64) {
        let distance = self.mahalanobis_distance(row); 
        (distance > self.threshold, distance)
    }

    pub fn get_mean(&self) -> &DVector<f64> {
        &self.mean
    }

    pub fn get_covariance_matrix(&self) -> &DMatrix<f64> {
        &self.covariance_matrix
    }

    pub fn get_threshold(&self) -> f64 {
        self.threshold
    }

    pub fn get_counter(&self) -> usize {
        self.counter
    }

    pub fn get_last_timestamp(&self) -> u128 {
        self.last_timestamp
    }

    //pub fn get_variance(&self) -> &DVector<f64> {
    //    &self.variance
    //}
}