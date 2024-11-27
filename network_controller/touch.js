//const fs = require('fs');
import * as fs from 'fs';

const transform = () => {
    // Read the CSV file
    const inputFilePath = 'transformed_gateway_with_jammer_status.csv';
    const outputFilePath = 'filtered_output.csv';
    const csv = fs.readFileSync(inputFilePath, 'utf8');
    
    // Split the content by newline characters to get an array of rows
    const rows = csv.split('\n');
    
    // Filter rows where the value in the 3rd column is '202'
    const filteredRows = rows.filter(row => {
        const columns = row.split(',');
        return columns[2] === '202' && Number(columns[7]) < 14;
    });

    
    
    // Join the filtered rows back into a single string
    const filteredCsv = filteredRows.join('\n');
    
    // Write the filtered rows to a new file
    fs.writeFileSync(outputFilePath, filteredCsv, 'utf8');
    
    console.log(`Filtered rows written to ${outputFilePath}`);
}

//read the content of output_anomalies.csv and output how many values in the third column are below 20 and how many are above 20 in the absolute value
const count_anomalies = () => {
    const filePath = 'output_anomalies.csv';
    const csv = fs.readFileSync(filePath, 'utf8');
    
    const rows = csv.split('\n');
    let below20 = 0;
    let above20 = 0;
    
    rows.forEach(row => {
        const columns = row.split(',');
        const value = Math.abs(Number(columns[2]));
        if (value < 20) {
            below20++;
        } else {
            above20++;
        }
    });
    
    console.log(`Number of values below 20: ${below20}`);
    console.log(`Number of values above 20: ${above20}`);
}

const count_normalities = () => {
    const filePath = 'output_normal.csv';
    const csv = fs.readFileSync(filePath, 'utf8');
    
    const rows = csv.split('\n');
    let below16 = 0;
    let above16 = 0;
    
    rows.forEach(row => {
        const columns = row.split(',');
        const value = Math.abs(Number(columns[2]));
        if (value < 16) {
            below16++;
        } else {
            above16++;
        }
    });
    
    console.log(`Number of values below 16: ${below16}`);
    console.log(`Number of values above 16: ${above16}`);
}

