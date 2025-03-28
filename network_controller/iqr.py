import numpy as np
import pandas as pd
import matplotlib.pyplot as plt
import seaborn as sns
from sklearn.metrics import accuracy_score, precision_score, recall_score, f1_score, confusion_matrix

def load_data(file_path):
    """
    Load CSV data into a pandas DataFrame.

    Parameters:
    - file_path: Path to the CSV file.

    Returns:
    - DataFrame containing the loaded data.
    """
    columns = ['Time', 'GatewayNode', 'Frequency', 'SF', 'RSSI', 'SNR', 'Distance', 'JammerActive']
    df = pd.read_csv(file_path, names=columns, skiprows=1)
    return df

def aggregate_packets(received_packets):
    """
    Aggregate the packets into 30-minute intervals.

    Parameters:
    - received_packets: DataFrame containing packet data.

    Returns:
    - Series containing packet counts aggregated by 30-minute intervals.
    """
    packet_df = received_packets.copy()
    packet_df['Time'] = pd.to_datetime(packet_df['Time'], unit='s')
    packet_df.set_index('Time', inplace=True)
    packet_agg = packet_df.groupby(pd.Grouper(freq='30min')).size()
    return packet_agg

def iqr_anomaly_detector(data, window_size=10, iqr_multiplier=1.5):
    """
    Detect anomalies in the data using the IQR method with a moving window, excluding anomalies from future calculations.

    Parameters:
    - data: Array-like numerical values (list, numpy array, pandas Series).
    - window_size: The size of the moving window for calculating Q1, Q3, and IQR.
    - iqr_multiplier: The multiplier for IQR to determine the threshold for anomalies.

    Returns:
    - anomalies: A list of indices where anomalies are detected.
    - thresholds: A list of upper and lower thresholds for each point.
    """
    data = np.asarray(data)
    anomalies = []
    thresholds = []
    non_anomalous_data = list(data[:window_size])  # Start with initial window data

    for i in range(window_size, len(data)):
        window_data = non_anomalous_data[-window_size:]

        if len(window_data) < window_size:
            # Not enough data to calculate IQR
            continue

        q1 = np.percentile(window_data, 25)
        q3 = np.percentile(window_data, 75)
        iqr = q3 - q1

        lower_bound = q1 - iqr_multiplier * iqr
        upper_bound = q3 + iqr_multiplier * iqr
        thresholds.append((lower_bound, upper_bound))

        if data[i] < lower_bound or data[i] > upper_bound:
            anomalies.append(i)
        else:
            non_anomalous_data.append(data[i])

    return anomalies, thresholds

def calculate_metrics(true_labels, predicted_labels):
    """
    Calculate accuracy, precision, recall, and F1-score.

    Parameters:
    - true_labels: Ground truth binary labels (list or numpy array).
    - predicted_labels: Predicted binary labels (list or numpy array).

    Returns:
    - A dictionary containing accuracy, precision, recall, F1-score, and confusion matrix.
    """
    accuracy = accuracy_score(true_labels, predicted_labels)
    precision = precision_score(true_labels, predicted_labels, zero_division=0)
    recall = recall_score(true_labels, predicted_labels, zero_division=0)
    f1 = f1_score(true_labels, predicted_labels, zero_division=0)
    conf_matrix = confusion_matrix(true_labels, predicted_labels)

    return {
        "accuracy": accuracy,
        "precision": precision,
        "recall": recall,
        "f1_score": f1,
        "confusion_matrix": conf_matrix
    }

# Main script
if __name__ == "__main__":
    # Parameter settings
    min_window_size = 5
    max_window_size = 25
    min_iqr_multiplier = 1.0
    max_iqr_multiplier = 3.0

    # Load the data from CSV file
    file_path = "4week.csv"  # Replace with your actual file path
    received_packets = load_data(file_path)

    # Aggregate packets into 30-minute intervals
    packet_counts = aggregate_packets(received_packets)

    # Extract counts from the Series
    counts = packet_counts.values

    # Prepare true labels for evaluation
    received_packets['Time'] = pd.to_datetime(received_packets['Time'], unit='s')
    received_packets.set_index('Time', inplace=True)
    true_labels = received_packets.groupby(pd.Grouper(freq='30min'))['JammerActive'].max().reindex(packet_counts.index, fill_value=0).values

    best_window_size = None
    best_iqr_multiplier = None
    best_accuracy = 0
    best_metrics = None
    results = []

    # Test different window sizes and IQR multipliers
    for window_size in range(min_window_size, max_window_size + 1):
        for iqr_multiplier in np.arange(min_iqr_multiplier, max_iqr_multiplier + 0.1, 0.1):
            # Apply the anomaly detector to the counts with a moving window approach
            anomaly_indices, thresholds = iqr_anomaly_detector(counts, window_size=window_size, iqr_multiplier=iqr_multiplier)

            # Prepare predicted labels
            predicted_labels = np.zeros_like(true_labels)
            predicted_labels[anomaly_indices] = 1

            # Calculate evaluation metrics
            metrics = calculate_metrics(true_labels, predicted_labels)

            # Store the results for plotting
            results.append({
                "window_size": window_size,
                "iqr_multiplier": iqr_multiplier,
                "accuracy": metrics["accuracy"],
                "precision": metrics["precision"]
            })

            # Check if this combination gives better accuracy
            if metrics["accuracy"] > best_accuracy:
                best_accuracy = metrics["accuracy"]
                best_window_size = window_size
                best_iqr_multiplier = iqr_multiplier
                best_metrics = metrics

    # Print the best combination of window size and IQR multiplier and corresponding evaluation metrics
    print(f"Best Window Size: {best_window_size}, Best IQR Multiplier: {best_iqr_multiplier}")
    print("Best Evaluation Metrics:")
    for metric, value in best_metrics.items():
        print(f"{metric}: {value}")

    # Apply the anomaly detector using the best combination
    anomaly_indices, thresholds = iqr_anomaly_detector(counts, window_size=best_window_size, iqr_multiplier=best_iqr_multiplier)

    # Map anomalies back to their timestamps
    anomalous_timestamps = packet_counts.index[anomaly_indices]
    anomalous_values = counts[anomaly_indices]

    # Visualize the data and detected anomalies
    plt.figure(figsize=(12, 6))
    plt.plot(packet_counts.index, packet_counts.values, label='Packet Counts')
    plt.scatter(anomalous_timestamps, anomalous_values, color='red', label='Anomalies')
    plt.xlabel('Time')
    plt.ylabel('Packet Count')
    plt.title('Packet Counts with Anomalies')
    plt.legend()
    plt.show(block=False)

    # Plot how precision and accuracy change with window size and IQR multiplier
    results_df = pd.DataFrame(results)
    pivot_acc = results_df.pivot(index='window_size', columns='iqr_multiplier', values='accuracy')
    pivot_prec = results_df.pivot(index='window_size', columns='iqr_multiplier', values='precision')

    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(24, 6))

    ax1.set_title('Accuracy by Window Size and IQR Multiplier')
    ax1.set_xlabel('IQR Multiplier')
    ax1.set_ylabel('Window Size')
    cax1 = ax1.imshow(pivot_acc, aspect='auto', cmap='viridis', origin='lower', extent=[min_iqr_multiplier, max_iqr_multiplier, min_window_size, max_window_size])
    fig.colorbar(cax1, ax=ax1, label='Accuracy')
    ax1.set_xlim(min_iqr_multiplier, max_iqr_multiplier)
    ax1.set_ylim(min_window_size, max_window_size)

    ax2.set_title('Precision by Window Size and IQR Multiplier')
    ax2.set_xlabel('IQR Multiplier')
    ax2.set_ylabel('Window Size')
    cax2 = ax2.imshow(pivot_prec, aspect='auto', cmap='viridis', origin='lower', extent=[min_iqr_multiplier, max_iqr_multiplier, min_window_size, max_window_size])
    fig.colorbar(cax2, ax=ax2, label='Precision')
    ax2.set_xlim(min_iqr_multiplier, max_iqr_multiplier)
    ax2.set_ylim(min_window_size, max_window_size)

    plt.tight_layout()
    plt.show(block=False)

    # Plot confusion matrix for the best combination
    conf_matrix = best_metrics['confusion_matrix']
    plt.figure(figsize=(8, 6))
    sns.heatmap(conf_matrix, annot=True, fmt='d', cmap='Blues', cbar=False)
    plt.title('Best Confusion Matrix')
    plt.xlabel('Predicted Label')
    plt.ylabel('True Label')
    plt.show(block=False)

    # Test the trained model using a new dataset
    test_file_path = "4week_test_.csv"  # Replace with your test file path
    test_received_packets = load_data(test_file_path)
    test_packet_counts = aggregate_packets(test_received_packets)
    test_counts = test_packet_counts.values

    # Apply the trained model to the test dataset using the same iqr_anomaly_detector function
    test_anomaly_indices, test_thresholds = iqr_anomaly_detector(test_counts, window_size=best_window_size, iqr_multiplier=best_iqr_multiplier)

    # Prepare true labels for the test dataset
    test_received_packets['Time'] = pd.to_datetime(test_received_packets['Time'], unit='s')
    test_received_packets.set_index('Time', inplace=True)
    test_true_labels = test_received_packets.groupby(pd.Grouper(freq='30min'))['JammerActive'].max().reindex(test_packet_counts.index, fill_value=0).values

    # Prepare predicted labels for the test dataset
    test_predicted_labels = np.zeros_like(test_true_labels)
    test_predicted_labels[test_anomaly_indices] = 1

    # Calculate evaluation metrics for the test dataset
    test_metrics = calculate_metrics(test_true_labels, test_predicted_labels)
    print("Test Evaluation Metrics:")
    for metric, value in test_metrics.items():
        print(f"{metric}: {value}")

    # Map test anomalies back to their timestamps
    test_anomalous_timestamps = test_packet_counts.index[test_anomaly_indices]
    test_anomalous_values = test_counts[test_anomaly_indices]

    # Visualize the test data and detected anomalies
    plt.figure(figsize=(12, 6))
    plt.plot(test_packet_counts.index, test_packet_counts.values, label='Packet Counts')
    plt.scatter(test_anomalous_timestamps, test_anomalous_values, color='red', label='Anomalies')
    plt.xlabel('Time')
    plt.ylabel('Packet Count')
    plt.title('Test Packet Counts with Anomalies')
    plt.legend()
    plt.show(block=False)

    # Plot confusion matrix for the best combination
    test_conf_matrix = test_metrics['confusion_matrix']
    plt.figure(figsize=(8, 6))
    sns.heatmap(test_conf_matrix, annot=True, fmt='d', cmap='Blues', cbar=False)
    plt.title('Test Confusion Matrix')
    plt.xlabel('Predicted Label')
    plt.ylabel('True Label')
    plt.show(block=False)

    input("Press Enter to exit...")  # Keep the plots open until user exits