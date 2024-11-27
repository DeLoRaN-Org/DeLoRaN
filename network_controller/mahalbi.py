import pandas as pd
import numpy as np
from scipy.spatial.distance import mahalanobis
from scipy.stats import chi2
from sklearn.preprocessing import StandardScaler

def read_csv_rows(file_path, include_anomalies=False):
    with open(file_path, 'r') as f:
        lines = f.readlines()
        
    rows = []
    is_anomaly = []
    #last_timestamps = {}
    file_mixed = open('transformed_filtered.csv', 'w')
    file_training = open('transformed_filtered_normal.csv', 'w')

    file_anomalies = open('anomalies.csv', 'w')
    file_normal = open('normals.csv', 'w')

    for i in range(1, len(lines)):
        line = lines[i].strip().split(',')
        timestamp = float(line[0])  # milliseconds since the start of the simulation
        anomaly = line[9] == "1"
        distance = line[8]
        freq = line[3]
        sf = line[4]
        rssi = line[6]
        snr = line[7]

        is_anomaly.append(anomaly)

        #calculate the ration between snr and distance and write it to the correct file based on anomaly
        if i < 500:
            file_training.write(f"{sf}, {freq}, {abs((float(snr) / float(distance)) * 100000000)}, {anomaly} \n")
        else:
            file_mixed.write(f"{sf}, {freq}, {abs((float(snr) / float(distance)) * 100000000)}, {anomaly} \n")

        if anomaly:
            file_anomalies.write(f"{sf}, {freq}, {abs((float(snr) / float(distance)) * 100000000)}, {anomaly} \n")
        else: 
            file_normal.write(f"{sf}, {freq}, {abs((float(snr) / float(distance)) * 100000000)}, {anomaly} \n")


        #key = (sf, freq)
        #if key in last_timestamps:
        #    delay = (timestamp - last_timestamps[key]) / 1000.0  # convert to seconds
        #else:
        #    delay = 0
        #last_timestamps[key] = timestamp



        if not anomaly or include_anomalies:
            #rows.append([sf, freq, rssi, snr, delay])
            rows.append([sf, freq, rssi, snr, distance])
    
    from sys import exit
    exit()


    df = pd.DataFrame(rows, columns=['sf', 'freq', 'rssi', 'snr', 'distance'])
    return df, is_anomaly

def train_mahalanobis(df):
    df = df.apply(pd.to_numeric, errors='coerce')
    df.dropna(inplace=True)

    grouped = df.groupby(['sf', 'freq'])
    ret = {}

    for (sf, freq), group in grouped:
        scaler = StandardScaler()
        data_scaled = scaler.fit_transform(group[['rssi', 'snr', 'distance']])
        
        cov_matrix = np.cov(data_scaled, rowvar=False)
        cov_matrix_inv = np.linalg.inv(cov_matrix)
        mean = np.mean(data_scaled, axis=0)

        ret[(sf, freq)] = (mean, cov_matrix_inv)

    return ret

def evaluate_mahalanobis(df, mahalanobis_dict, is_anomaly, threshold=0.99):
    df = df.apply(pd.to_numeric, errors='coerce')
    df.dropna(inplace=True)

    results = []
    grouped = df.groupby(['sf', 'freq'])
    j = 0

    distances = []

    for (sf, freq), group in grouped:
        mean, cov_matrix_inv = mahalanobis_dict[(sf, freq)]
        scaler = StandardScaler()
        data_scaled = scaler.fit_transform(group[['rssi', 'snr', 'distance']])

        mahalanobis_distances = []
        for i in range(data_scaled.shape[0]):
            distance = mahalanobis(data_scaled[i], mean, cov_matrix_inv)
            mahalanobis_distances.append(distance)

        chi_squared_threshold = chi2.ppf(threshold, data_scaled.shape[1])
        
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
            'sf': sf,
            'freq': freq,
            'correct_anomalies': len(correct_anomalies),
            'wrong_anomalies': len(wrong_anomalies),
            'correct_valid': len(correct_valid),
            'wrong_valid': len(wrong_valid)
        })

    return results

def main():
    file_path = './filtered_output.csv'

    df_clear, _ = read_csv_rows(file_path, include_anomalies=False)
    df_with_anomaly, is_anomaly = read_csv_rows(file_path, include_anomalies=True)

    mahalanobis = train_mahalanobis(df_clear)
    results = evaluate_mahalanobis(df_with_anomaly, mahalanobis, is_anomaly)

    for result in filter(lambda x: x['correct_anomalies'] > 0 or x['wrong_valid'] > 0, results):
        print(f"SF: {result['sf']}, Freq: {result['freq']}")
        print("Correct Anomalies: ", result['correct_anomalies'])
        print("Wrong Anomalies: ", result['wrong_anomalies'])
        print("Correct Valid: ", result['correct_valid'])
        print("Wrong Valid: ", result['wrong_valid'])
        precision = result['correct_anomalies'] / (result['correct_anomalies'] + result['wrong_anomalies'] + 1)
        recall = result['correct_anomalies'] / (result['correct_anomalies'] + result['wrong_valid'] + 1)
        f1 = 2 * (precision * recall) / (precision + recall + 1)
        accuracy = (result['correct_anomalies'] + result['correct_valid']) / (result['correct_anomalies'] + result['correct_valid'] + result['wrong_anomalies'] + result['wrong_valid'])
        print("Precision: ", precision)
        print("Recall: ", recall)
        print("F1 Score: ", f1)
        print("Accuracy: ", accuracy)
        print("\n")

if __name__ == "__main__":
    main()