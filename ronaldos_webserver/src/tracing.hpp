#pragma once
#include <format>
#include <string>

extern "C" {
void log_error(const char *);
void log_info(const char *);
}

#define DEFINE_LOG_FUNCTION(level)                                             \
  inline void level(const std::string_view data) { log_##level(data.data()); } \
  template <typename... Args>                                                  \
  inline void level(const std::format_string<Args...> fmt, Args &&...args) {   \
    auto message = std::vformat(fmt.get(), std::make_format_args(args...));    \
    log_##level(message.data());                                               \
  }

DEFINE_LOG_FUNCTION(error)
DEFINE_LOG_FUNCTION(info)
