import pandas as pd
import numpy as np
from scipy.spatial.distance import mahalanobis
from scipy.stats import chi2
from sklearn.preprocessing import StandardScaler

def read_csv_rows(file_path):
    with open(file_path, 'r') as f:
        lines = f.readlines()
        
    rows = []
    is_anomaly = [] 
    for i in range(1, len(lines)):
        line = lines[i].strip().split(',')
        sf = line[0]
        freq = line[1]
        anomalities = line[2]
        rows.append([sf, freq, anomalities])

        is_anomaly.append(line[3].strip() == "True")
    df = pd.DataFrame(rows, columns=['sf','freq','anomalities'])
    return df, is_anomaly

def train_mahalanobis(df):
    df = df.apply(pd.to_numeric, errors='coerce')
    df.dropna(inplace=True)

    #grouped = df.groupby(['sf', 'freq'])
    #ret = {}

    scaler = StandardScaler()
    data_scaled = scaler.fit_transform(df[['anomalities']])
    df['anomalities'] = data_scaled

    cov_matrix = np.cov(df, rowvar=False)
    cov_matrix_inv = np.linalg.inv(cov_matrix)
    mean = np.mean(df, axis=0)
    return mean, cov_matrix_inv

def evaluate_mahalanobis(df, mean, cov_matrix_inv, is_anomaly, threshold=0.99):
    df = df.apply(pd.to_numeric, errors='coerce')
    df.dropna(inplace=True)

    results = []
    j = 0

    distances = []

    scaler = StandardScaler()
    data_scaled = scaler.fit_transform(df[['anomalities']])
    df['anomalities'] = data_scaled


    mahalanobis_distances = []
    for i in range(df.shape[0]):
        distance = mahalanobis(df.iloc[i], mean, cov_matrix_inv)
        mahalanobis_distances.append(distance)

    chi_squared_threshold = chi2.ppf(threshold, df.shape[1])
    
    correct_anomalies = []
    wrong_anomalies = []
    correct_valid = []
    wrong_valid = []

    for i in range(len(mahalanobis_distances)):
        distances.append(f"{j}, {mahalanobis_distances[i]}, {is_anomaly[i]}, {mahalanobis_distances[i] > np.sqrt(chi_squared_threshold)}")
        j += 1
        if mahalanobis_distances[i] > np.sqrt(chi_squared_threshold):
            if is_anomaly[i]:
                correct_anomalies.append(mahalanobis_distances[i])
            else:
                wrong_anomalies.append(mahalanobis_distances[i])
        else:
            if is_anomaly[i]:
                wrong_valid.append(mahalanobis_distances[i])
            else:
                correct_valid.append(mahalanobis_distances[i])


    with open('output.csv', 'w') as file:
        file.write("\n".join(distances))


    results.append({
        'correct_anomalies': len(correct_anomalies),
        'wrong_anomalies': len(wrong_anomalies),
        'correct_valid': len(correct_valid),
        'wrong_valid': len(wrong_valid)
    })

    return results

def main():
    file_path = './filtered_output.csv'

    df_clear, _ = read_csv_rows('./transformed_filtered_normal.csv')
    df_with_anomaly, is_anomaly = read_csv_rows('./transformed_filtered.csv')

    mean, inv_cov = train_mahalanobis(df_clear)
    results = evaluate_mahalanobis(df_with_anomaly, mean, inv_cov, is_anomaly)

    for result in results: #filter(lambda x: x['correct_anomalies'] > 0 or x['wrong_valid'] > 0, results):
        print(result)
        #print(f"SF: {result['sf']}, Freq: {result['freq']}")
        #print("Correct Anomalies: ", result['correct_anomalies'])
        #print("Wrong Anomalies: ", result['wrong_anomalies'])
        #print("Correct Valid: ", result['correct_valid'])
        #print("Wrong Valid: ", result['wrong_valid'])
        #precision = result['correct_anomalies'] / (result['correct_anomalies'] + result['wrong_anomalies'] + 1)
        #recall = result['correct_anomalies'] / (result['correct_anomalies'] + result['wrong_valid'] + 1)
        #f1 = 2 * (precision * recall) / (precision + recall + 1)
        #accuracy = (result['correct_anomalies'] + result['correct_valid']) / (result['correct_anomalies'] + result['correct_valid'] + result['wrong_anomalies'] + result['wrong_valid'])
        #print("Precision: ", precision)
        #print("Recall: ", recall)
        #print("F1 Score: ", f1)
        #print("Accuracy: ", accuracy)
        #print("\n")

if __name__ == "__main__":
    main()