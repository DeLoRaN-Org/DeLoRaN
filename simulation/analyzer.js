const { writeFileSync, readFileSync } = require('fs');
let { ChartJSNodeCanvas } = require('chartjs-node-canvas');


const average = (array) => array.reduce((a, b) => a + b) / array.length;

const simpleMovingAverage = (prices, interval) => {
    let index = interval - 1;
    const length = prices.length + 1;
    let results = [];

    while (index < length) {
        index = index + 1;
        const intervalSlice = prices.slice(index - interval, index);
        const sum = intervalSlice.reduce((prev, curr) => prev + curr, 0);
        results.push(sum / interval);
    }

    return results;
}

function plotSingleChart() {
    const chartJSNodeCanvas = new ChartJSNodeCanvas({ type: 'pdf', width: 800, height: 600, backgroundColour: 'white' });

    for (let index = 1; index <= 16; index++) {
        let rows = readFileSync(`./cpu_usages/nc${index.toString().padStart(2, '0')}_cpu_mem_usage.csv`, 'utf8').toString().split('\n');
        rows.shift()
        let labels = []
        let cpu = []
        let mem = []

        let i = 0
        for (let row of rows) {
            let [cpu_usage, mem_usage] = row.split(',').map(Number)
            if (cpu_usage > 30) {
                cpu_usage = 30
            }
            cpu.push((cpu_usage / 30) * 100)
            mem.push(mem_usage)
            labels.push(i++)
        }


        let cpu_average = simpleMovingAverage(cpu, 6)

        const configuration_cpu = {
            type: 'bar',
            data: {
                labels,
                datasets: [
                    {
                        label: 'Cpu usage',
                        data: cpu_average,
                        backgroundColor: "#0000ff",
                    },
                ],
            },
            options: {
                scales: {
                    y: {
                        min: 0,
                        max: 100,
                    }
                },
            }           
        };

        const configuration_mem = {
            type: 'bar',
            data: {
                labels,
                datasets: [
                    {
                        label: 'Mem usage',
                        data: mem,
                        backgroundColor: "#ff0000",
                    },
                ],
            },
            options: {
                scales: {
                    y: {
                        min: 0,
                        max: 1000,
                    }
                },
            }
        };

        let image_cpu = chartJSNodeCanvas.renderToBufferSync(configuration_cpu)
        writeFileSync(`./plots/chart_cpu_nc${index.toString().padStart(2, '0')}.pdf`, image_cpu);
        let image_mem = chartJSNodeCanvas.renderToBufferSync(configuration_mem)
        writeFileSync(`./plots/chart_mem_nc${index.toString().padStart(2, '0')}.pdf`, image_mem);
    }
}


function plotOverallAverageChart() {
    let total_cpu = null
    let total_mem = null

    for (let index = 1; index <= 16; index++) {
        let rows = readFileSync(`./cpu_usages/nc${index.toString().padStart(2, '0')}_cpu_mem_usage.csv`, 'utf8').toString().split('\n');
        rows.shift()
        let cpu = []
        let mem = []

        let i = 0
        for (let row of rows) {
            let [cpu_usage, mem_usage] = row.split(',').map(Number)
            if (cpu_usage > 30) {
                cpu_usage = 30
            }
            cpu.push((cpu_usage / 30) * 100)
            mem.push(mem_usage)
        }


        let cpu_moving_average = simpleMovingAverage(cpu, 6)

        if (total_cpu == null) {
            total_cpu = cpu_moving_average
        } else {
            total_cpu = total_cpu.map((num, idx) => num + cpu_moving_average[idx])
        }

        if (total_mem == null) {
            total_mem = mem
        } else {
            total_mem = total_mem.map((num, idx) => num + mem[idx])
        }
    }


    let cpu_average = total_cpu.map(num => num / 16)
    let mem_average = total_mem.map(num => num / 16)

    cpu_average = cpu_average.filter((v) => !isNaN(v))
    mem_average = mem_average.filter((v) => !isNaN(v))


    const chartJSNodeCanvas = new ChartJSNodeCanvas({ type: 'pdf', width: 800, height: 600, backgroundColour: 'white' });

    const configuration_cpu = {
        type: 'bar',
        data: {
            labels: cpu_average.map((_, i) => i),
            datasets: [
                {
                    label: 'Cpu usage',
                    data: cpu_average,
                    backgroundColor: "#0000ff",
                },
            ],
        },
        options: {
            scales: {
                y: {
                    min: 0,
                    max: 100,
                }
            },
        }
    };

    const configuration_mem = {
        type: 'bar',
        data: {
            labels: mem_average.map((_, i) => i),
            datasets: [
                {
                    label: 'Mem usage',
                    data: mem_average,
                    backgroundColor: "#ff0000",
                },
            ],
        },
        options: {
            scales: {
                y: {
                    min: 0,
                    max: 1000,
                }
            },
        }
    };

    let image_cpu = chartJSNodeCanvas.renderToBufferSync(configuration_cpu)
    writeFileSync(`./plots/chart_cpu_average.pdf`, image_cpu);
    let image_mem = chartJSNodeCanvas.renderToBufferSync(configuration_mem)
    writeFileSync(`./plots/chart_mem_average.pdf`, image_mem);
}



function plotChirpstackChart() {
    const chartJSNodeCanvas = new ChartJSNodeCanvas({ type: 'pdf', width: 1600, height: 1200, backgroundColour: 'white' });
    
    let rows = readFileSync(`./output/chirpstack_rtt_d1000_p100_1719498319623.csv`, 'utf8').toString().split('\n');
    rows.shift()

    let labels = []
    let rtts = []

    let i = 0
    for (let row of rows) {
        let [_, rtt, tmst] = row.split(',').map(Number)
        //if (cpu_usage > 30) {
        //    cpu_usage = 30
        //}
        //cpu.push((cpu_usage / 30) * 100)
        //mem.push(mem_usage)
        labels.push(tmst)
        rtts.push(rtt)
    }

    let rtt_average = simpleMovingAverage(rtts, 6)

    const configuration_rtt = {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Rtt',
                    data: rtt_average,
                    backgroundColor: "#0000ff",
                },
            ],
        },
        //options: {
        //    scales: {
        //        y: {
        //            min: 0,
        //            max: 100,
        //        }
        //    },
        //}           
    };

    let image_rtt = chartJSNodeCanvas.renderToBufferSync(configuration_rtt)
    writeFileSync(`./chirpstack_plots/rtts.pdf`, image_rtt);


    rows = readFileSync(`./output/chirpstack_simulation_stats_1719498319624.csv`, 'utf8').toString().split('\n');
    rows.shift()

    labels = []
    let cpus = []
    let mems = []
    let n_is = []
    let n_os = []

    for (let row of rows) {
        let [tmst, name, cpu, ram, n_i, n_o] = row.split(',').map((v, i) => {
            if (i == 0) {
                return Math.round(Number(v) / 60000)
            } else if (i == 1) {
                return v.replace("chirpstack-docker-", "")
            } else if (i == 2) {
                return Number(v.replace("%", ""))
            } else if (i == 3) {
                return Number(v.replace("MiB", ""))
            } else  {
                return Number(v)
            }
        })

        if(name != "chirpstack-1") continue;
        labels.push(tmst)
        cpus.push(cpu)
        mems.push(ram)
        n_is.push(n_i)
        n_os.push(n_o)
    }

    n_is.shift()
    n_os.shift()

    let total_ni = n_is.reduce((a, b) => a + b, 0)
    let total_no = n_os.reduce((a, b) => a + b, 0)

    let cpu_average = simpleMovingAverage(cpus, 6)
    let mem_average = simpleMovingAverage(mems, 6)
    let n_i_average = simpleMovingAverage(n_is, 1)
    let n_o_average = simpleMovingAverage(n_os, 1)


    const configuration_cpu = {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Cpu usage',
                    data: cpu_average,
                    backgroundColor: "#0000ff",
                },
            ],
        },
        options: {
            scales: {
                y: {
                    min: 0,
                    max: 100,
                }
            },
        }           
    };

    const configuration_ni = {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Network Input',
                    data: n_i_average,
                    backgroundColor: "#0000ff",
                },
            ],
        },
    };
    
    const configuration_no = {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Network Output',
                    data: n_o_average,
                    backgroundColor: "#0000ff",
                },
            ],
        },
    };

    const configuration_mem = {
        type: 'bar',
        data: {
            labels,
            datasets: [
                {
                    label: 'Mem usage',
                    data: mem_average,
                    backgroundColor: "#ff0000",
                },
            ],
        },
        //options: {
        //    scales: {
        //        y: {
        //            min: 0,
        //            max: 1000,
        //        }
        //    },
        //}
    };



    let image_cpu = chartJSNodeCanvas.renderToBufferSync(configuration_cpu)
    writeFileSync(`./chirpstack_plots/cpu.pdf`, image_cpu);

    let image_ni = chartJSNodeCanvas.renderToBufferSync(configuration_ni)
    writeFileSync(`./chirpstack_plots/ni.pdf`, image_ni);

    let image_no = chartJSNodeCanvas.renderToBufferSync(configuration_no)
    writeFileSync(`./chirpstack_plots/no.pdf`, image_no);

    let image_mem = chartJSNodeCanvas.renderToBufferSync(configuration_mem)
    writeFileSync(`./chirpstack_plots/mem.pdf`, image_mem);



    console.log("Total Network Input: ", total_ni)
    console.log("Total Network Output: ", total_no)
}


//plotSingleChart()
//plotOverallAverageChart()

plotChirpstackChart()