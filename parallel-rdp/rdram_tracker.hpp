#pragma once

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <vector>

// Tracks RDRAM that parallel-rdp may still read or write on the GPU, so an
// emulated access that overlaps it can wait first.
//
// Every 8-byte block holds a stamp: the timeline value that covers the last
// RDP command reading or writing it (0 if never touched). The tracker knows
// nothing about the GPU; callers pass the stamp when marking, and compare the
// stamp returned by conflict() against the GPU's timeline. Stamps never need
// clearing: once the GPU completes a value, every block stamped with it or
// earlier is free.
class RdramTracker {
public:
  enum class Access { Read, Write };

  // forget all stamps, e.g. when the timeline starts over; rdram_size fixes
  // the trackable range
  void reset(size_t rdram_size) {
    write_stamps.assign(rdram_size >> 3, 0);
    read_stamps.assign(rdram_size >> 3, 0);
    newest = 0;
  }

  // the highest stamp ever marked; once the GPU completes it nothing can
  // conflict
  uint64_t max_stamp() const { return newest; }

  // the RDP accesses [address, address + length) in commands covered by
  // timeline value `stamp`. Stamps only grow, so a later mark always wins.
  void mark(uint64_t address, uint64_t length, Access access, uint64_t stamp) {
    size_t first, last;
    if (!block_range(address, length, first, last))
      return;
    // timeline values stay far below 2^32 in practice (about one per SyncFull)
    uint32_t s = uint32_t(stamp);
    newest = std::max(newest, s);
    auto &stamps = access == Access::Write ? write_stamps : read_stamps;
    std::fill(stamps.begin() + first, stamps.begin() + last + 1, s);
  }

  // the RDP accesses pixels [x0, x1] x [y0, y1] of an image `width` pixels
  // wide at `base`
  void mark_rect(uint32_t base, uint32_t width, uint32_t bytes_per_pixel,
                 int32_t x0, int32_t x1, int32_t y0, int32_t y1, Access access,
                 uint64_t stamp) {
    if (x1 < x0 || y1 < y0)
      return;
    uint64_t stride = uint64_t(width) * bytes_per_pixel;
    if (x0 == 0 && uint32_t(x1) + 1 == width) {
      mark(base + y0 * stride, (y1 - y0 + 1) * stride, access, stamp);
      return;
    }
    for (int32_t y = y0; y <= y1; y++)
      mark(base + y * stride + uint64_t(x0) * bytes_per_pixel,
           uint64_t(x1 - x0 + 1) * bytes_per_pixel, access, stamp);
  }

  // The timeline value an emulated access to [address, address + length) has
  // to wait for, 0 if none. Emulated reads conflict with RDP writes; emulated
  // writes conflict with RDP reads and writes.
  uint64_t conflict(uint64_t address, uint64_t length, bool write) const {
    size_t first, last;
    if (!block_range(address, length, first, last))
      return 0;
    uint32_t stamp = *std::max_element(write_stamps.begin() + first,
                                       write_stamps.begin() + last + 1);
    if (write)
      stamp =
          std::max(stamp, *std::max_element(read_stamps.begin() + first,
                                            read_stamps.begin() + last + 1));
    return stamp;
  }

private:
  // the 8-byte blocks [first, last] covering [address, address + length)
  bool block_range(uint64_t address, uint64_t length, size_t &first,
                   size_t &last) const {
    uint64_t blocks = write_stamps.size();
    first = address >> 3;
    if (length == 0 || first >= blocks)
      return false;
    last = std::min((address + length - 1) >> 3, blocks - 1);
    return true;
  }

  std::vector<uint32_t> write_stamps; // color / depth buffer writes
  std::vector<uint32_t> read_stamps;  // texture loads, depth compare
  uint32_t newest = 0;
};
