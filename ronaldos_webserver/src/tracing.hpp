#pragma once
#include <format>
#include <string>

#define DEFINE_LOG_FUNCTION(level)                                             \
  extern "C" {                                                                 \
  void ffi_##level(const char *);                                              \
  }                                                                            \
  inline void level(const std::string_view data) { ffi_##level(data.data()); } \
  template <typename... Args>                                                  \
  inline void level(const std::format_string<Args...> fmt, Args &&...args) {   \
    auto message = std::vformat(fmt.get(), std::make_format_args(args...));    \
    ffi_##level(message.data());                                               \
  }

DEFINE_LOG_FUNCTION(error)
DEFINE_LOG_FUNCTION(warn)
DEFINE_LOG_FUNCTION(info)
DEFINE_LOG_FUNCTION(debug)
