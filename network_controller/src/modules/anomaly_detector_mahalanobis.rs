use std::{collections::HashMap, time::Instant};

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
        let key = (sf, frequency);
        let detector = self.mahalnobis.entry(key).or_insert(Mahalnobis::new(self.threshold, self.num_features));
        detector.update(row);
    }

    pub fn update_rows(&mut self, sf: SpreadingFactor, frequency: String, data: &[DVector<f64>]) {
        let key = (sf, frequency);
        let detector = self.mahalnobis.entry(key).or_insert(Mahalnobis::new(self.threshold, self.num_features));
        detector.update_rows(data);
    }

    pub fn is_anomaly(&mut self, sf: SpreadingFactor, frequency: String, row: &DVector<f64>) -> (bool, f64) {
        let key = (sf, frequency);
        let detector = self.mahalnobis.entry(key).or_insert(Mahalnobis::new(self.threshold, self.num_features));
        detector.is_anomaly(row)
    }

    pub fn mean(&self, sf: SpreadingFactor, fq: String) -> &DVector<f64> {
        &self.mahalnobis[&(sf, fq)].mean
    }
    
    pub fn variance(&self, sf: SpreadingFactor, fq: String) -> &DVector<f64> {
        &self.mahalnobis[&(sf, fq)].variance
    }
    
    pub fn get_mahalanobi(&self, sf: SpreadingFactor, fq: String) -> &Mahalnobis {
        &self.mahalnobis[&(sf, fq)]
    }
    
    pub fn covariance_matrix(&self, sf: SpreadingFactor, fq: String) -> &DMatrix<f64> {
        &self.mahalnobis[&(sf, fq)].covariance_matrix
    }
}

#[derive(Debug)]
pub struct Mahalnobis {
    //id: String,
    mean: DVector<f64>,
    m2: DVector<f64>,
    variance: DVector<f64>,
    covariance_matrix: DMatrix<f64>,
    threshold: f64,
    counter: usize,
    last_timestamp: u128,
}

impl Mahalnobis {
    pub fn new(threshold: f64, num_features: usize) -> Self {
        Mahalnobis {
            //id: format!("{:?}", Instant::now()),
            mean: DVector::from_element(num_features, 0.0),
            m2: DVector::from_element(num_features, 0.0),
            variance: DVector::from_element(num_features, 1.0),
            covariance_matrix: DMatrix::from_element(num_features, num_features, 1e-5),
            threshold,
            counter: 0,
            last_timestamp: 0,
        }
    }

    pub fn update(&mut self, row: &DVector<f64>) {
        let tmst = row[2] as u128;

        let mut row_c = row.clone();
        row_c[2] -= self.last_timestamp as f64;

        //let normalized_row = self.normalize_row(&row_c);
        self.counter += 1;

        self.update_mean(&row_c);
        self.update_covariance(&row_c);
        
        self.last_timestamp = tmst;
    }

    pub fn normalize_row(&self, row: &DVector<f64>) -> DVector<f64> {
        let mut std_dev = self.variance.clone();
        std_dev.apply(|arg0: &mut f64| { *arg0 = f64::sqrt(*arg0); });
        (row - &self.mean).component_div(&std_dev)
    }

    pub fn update_rows(&mut self, data: &[DVector<f64>]) {
        for row in data {
            self.update(row);
        }
    }
    
    pub fn update_mean(&mut self, row: &DVector<f64>) {
        let n = self.counter as f64;
        let delta = row - &self.mean;

        self.mean += &delta / n;

        let delta2 = row - &self.mean;

        self.m2 += delta.component_mul(&delta2);

        if self.counter > 2 {
            self.variance = &self.m2 / n;
        }
    }

    pub fn update_covariance(&mut self, row: &DVector<f64>) {
        let n = self.counter as f64;
        let diff = row - &self.mean;
        let outer_product = &diff * diff.transpose() * (n + 1.0);
        self.covariance_matrix = outer_product / n;
    }

    pub fn mahalanobis_distance(&mut self, row: &DVector<f64>) -> f64 {
        //let normalized_row = self.normalize_row(row);
        
        let regularization = 1e-4;  // A small regularization constant
        let reg_cov_matrix = &self.covariance_matrix + DMatrix::<f64>::identity(self.covariance_matrix.nrows(), self.covariance_matrix.ncols()) * regularization;
        let inv_cov = match reg_cov_matrix.clone().try_inverse() {
            Some(inv) => inv,
            None => {
                println!("{:?}", &self.covariance_matrix);
                println!("{:?}", &self.covariance_matrix.determinant());
                panic!("Regularized covariance matrix should be invertible");
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

    pub fn get_variance(&self) -> &DVector<f64> {
        &self.variance
    }
}