let content = require('fs').readFileSync('anomaly_detector_mahalanobis_test.csv', 'utf8');

let lines = content.split('\n');

let data = [];

for (let i = 0; i < lines.length; i++) {
    if(lines[i].includes('true, ')) {
        console.log(lines[i]);    
    }
}