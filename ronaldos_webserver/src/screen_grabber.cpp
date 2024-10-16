#include "tracing.hpp"
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <linux/videodev2.h>
#include <optional>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <sys/ioctl.h>
#include <unistd.h>
#include <vector>

std::optional<std::filesystem::path>
find_video_path(const std::string &dev_name) {
  std::filesystem::path video4linux_dir("/sys/class/video4linux");

  if (!std::filesystem::exists(video4linux_dir)) {
    return std::nullopt;
  }

  for (const auto &entry :
       std::filesystem::directory_iterator(video4linux_dir)) {
    std::filesystem::path name_file = entry.path() / "name";

    std::ifstream name_stream(name_file);
    if (name_stream) {
      std::string name;
      std::getline(name_stream, name);

      info("found video input device: {} - {}", entry.path().c_str(), name);
      if (name.find(dev_name) != std::string::npos) {
        return entry.path() / "dev";
      }
    }
  }

  return std::nullopt;
}

extern "C" {
#include <fcntl.h>
#include <libavcodec/avcodec.h>
#include <libavcodec/codec.h>
#include <libavformat/avformat.h>
#include <libavutil/opt.h>

#define FRAME_WIDTH 1920
#define FRAME_HEIGHT 1080
#define FPS 60
#define SEGMENT_DURATION 10 // Segment duration in seconds

// Function to set the camera properties for 1080p @ 60fps
int set_camera_properties(int fd) {
  struct v4l2_format format;
  memset(&format, 0, sizeof(format));
  format.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  format.fmt.pix.width = FRAME_WIDTH;
  format.fmt.pix.height = FRAME_HEIGHT;
  format.fmt.pix.pixelformat = V4L2_PIX_FMT_YUYV;
  format.fmt.pix.field = V4L2_FIELD_INTERLACED;

  if (ioctl(fd, VIDIOC_S_FMT, &format) < 0) {
    error("Could not set format: {}", strerror(errno));
    return -1;
  }

  struct v4l2_streamparm streamparm;
  memset(&streamparm, 0, sizeof(streamparm));
  streamparm.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  streamparm.parm.capture.timeperframe.numerator = 1;
  streamparm.parm.capture.timeperframe.denominator = FPS;

  if (ioctl(fd, VIDIOC_S_PARM, &streamparm) < 0) {
    error("Could not set frame rate");
    return -1;
  }

  return 0;
}

uint32_t screen_grabber_init() {
  avformat_network_init();
  return 0;
}

uint32_t screen_grabber_instances_running() { return 0; }

int screen_grabber_start(const char *capture_device, const char *output) {
  if (!output || output[0] == '\0') {
    error("no output path defined to write captures to");
    return -1;
  }

  auto video_path = find_video_path(capture_device);
  if (!video_path) {
    error("could not find {}", capture_device);
    return -1;
  }

  // Open the video device
  int fd = open((*video_path).c_str(), O_RDWR);
  if (fd < 0) {
    error("Failed to open {}: {}", video_path->c_str(), strerror(errno));
    return 1;
  }

  // // Set camera properties to 1080p 60fps
  // if (set_camera_properties(fd) < 0) {
  //   close(fd);
  //   return 2;
  // }

  // Set up the output format context for HLS
  AVFormatContext *output_ctx = nullptr;
  avformat_alloc_output_context2(&output_ctx, nullptr, "hls", output);

  if (!output_ctx) {
    error("Could not create output context");
    close(fd);
    return 3;
  }

  // Open the output file
  if (!(output_ctx->oformat->flags & AVFMT_NOFILE)) {
    if (avio_open(&output_ctx->pb, output, AVIO_FLAG_WRITE) < 0) {
      error("Could not open output file {}", output);
      avformat_free_context(output_ctx);
      close(fd);
      return 4;
    }
  }

  // Video codec setup
  const AVCodec *codec = avcodec_find_encoder(AV_CODEC_ID_H264);
  if (!codec) {
    error("H264 codec not found");
    close(fd);
    return 5;
  }

  AVStream *video_st = avformat_new_stream(output_ctx, codec);
  if (!video_st) {
    error("Failed to create stream");
    avio_close(output_ctx->pb);
    avformat_free_context(output_ctx);
    close(fd);
    return 6;
  }

  AVCodecContext *codec_ctx = avcodec_alloc_context3(codec);
  codec_ctx->width = FRAME_WIDTH;
  codec_ctx->height = FRAME_HEIGHT;
  codec_ctx->time_base = {1, FPS};
  codec_ctx->framerate = {FPS, 1};
  codec_ctx->gop_size = 60; // Keyframe every 60 frames
  codec_ctx->pix_fmt = AV_PIX_FMT_YUV420P;
  codec_ctx->max_b_frames = 1;
  codec_ctx->bit_rate = 4000000;

  avcodec_parameters_from_context(video_st->codecpar, codec_ctx);

  if (avcodec_open2(codec_ctx, codec, nullptr) < 0) {
    std::cerr << "Could not open codec\n";
    avcodec_free_context(&codec_ctx);
    avio_close(output_ctx->pb);
    avformat_free_context(output_ctx);
    close(fd);
    return 7;
  }

  // Write the file header
  if (avformat_write_header(output_ctx, nullptr) < 0) {
    std::cerr << "Error occurred when writing header\n";
    avcodec_free_context(&codec_ctx);
    avio_close(output_ctx->pb);
    avformat_free_context(output_ctx);
    close(fd);
    return 8;
  }

  // Main loop to capture frames and encode
  for (;;) {
    // Capture video frame using V4L2 (simplified, add actual buffer management)
    // (Use OpenCV to get the frame from /dev/video0 or use direct V4L2 buffer
    // handling)

    // Encode the frame into HLS segment
    // (Insert frame processing, encoding, and HLS segment creation here)

    // Check for segmentation and close current segment after 10 seconds

    // Write the segment to the HLS playlist (e.g., output.m3u8)
  }

  // Close resources
  av_write_trailer(output_ctx);
  avcodec_free_context(&codec_ctx);
  avio_close(output_ctx->pb);
  avformat_free_context(output_ctx);

  close(fd);
  return 0;
}

int screen_grabber_stop_all() { return -1; }
}
