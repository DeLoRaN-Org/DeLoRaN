#!/bin/bash

while true; do
    cpu_usage=$(top -bn 1 | awk -v process_name="preloader" '$12 == process_name {print $9}')
    
    if [ -n "$cpu_usage" ]; then
        echo "CPU Utilization for 'preloader': $cpu_usage%"
    #else
    #    echo "'preloader' process not found."
    fi
    
    sleep 1
done
