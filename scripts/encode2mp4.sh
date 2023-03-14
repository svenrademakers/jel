#!/bin/bash

output="/tmp/recorded.mp4"

if [[ ! -z "$1" ]]; then
 output="$1" 
fi

ffmpeg\
       -thread_queue_size 16\
    -f v4l2 -thread_queue_size 4096 -i /dev/video0\
    -f pulse -i alsa_output.pci-0000_03_00.1.hdmi-stereo-extra4.monitor\
    -framerate 30 -pix_fmt yuv420p\
    -c:v libx264 -preset ultrafast -profile:v high -level:v 4.1 -b:v 14M\
    -c:a aac -b:a 192k\
    ${output}

