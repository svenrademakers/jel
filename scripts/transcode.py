import argparse
import subprocess
import os

parser = argparse.ArgumentParser()
parser.add_argument(dest='stream_name', help="name of the stream")
parser.add_argument(dest='description', help="title of the stream")
parser.add_argument('-cwd', nargs='?', default="Z:\\")
parser.add_argument('-d', "--datestamp", nargs='?', default=None)

args = parser.parse_args()

cwd= args.cwd
stream_name=args.stream_name
input_device=" -i video=\"Game Capture HD60 S+\":audio=\"Digital Audio Interface (Game Capture HD60 S+)\""

# if args.date is None:
#     date = datetime.datetime.now()
# else:
#     date = Date.parse(args.datestamp)

cmd= f'ffmpeg -f dshow -rtbufsize 2048M  {input_device} -r 30   \
-filter_complex \
"[0:v]split=3[v1][v2][v3]; \
[v1]format=yuv420p[v1out]; [v2]scale=w=1280:h=720, format=yuv420p [v2out]; [v3]scale=w=640:h=360, format=yuv420p[v3out]" \
-map [v1out] -c:v:0 libx264 -preset veryfast -b:v:0 14M -maxrate:v:0 14M -bufsize:v:0 14M \
-map [v2out] -c:v:1 libx264 -preset veryfast -b:v:1 6M -maxrate:v:1 6M -bufsize:v:1 6M \
-map [v3out] -c:v:2 libx264 -preset veryfast -b:v:2 1M -maxrate:v:2 1M -bufsize:v:2 1M \
-map a:0 -c:a:0 aac -b:a:0 96k -ac 2 \
-map a:0 -c:a:1 aac -b:a:1 96k -ac 2 \
-map a:0 -c:a:2 aac -b:a:2 48k -ac 2 \
-f hls \
-hls_time 2 \
-hls_list_size 4 \
-hls_flags independent_segments \
-hls_segment_type mpegts \
-hls_segment_filename .\{stream_name}\%v_%02d.ts \
-master_pl_name .\{stream_name}_vod.m3u8 \
-var_stream_map "v:0,a:0 v:1,a:1 v:2,a:2" .\{stream_name}\%v.m3u8 '

if not os.path.exists(os.path.join(cwd, stream_name)):
    os.makedirs(os.path.join(cwd, stream_name))

subprocess.run(cmd, shell=True, cwd=cwd)