#pragma once

#include <algorithm>
#include <bit>
#include <cstddef>
#include <cstdint>
#include <vector>

// Tracks RDRAM that parallel-rdp may still read or write on the GPU, so an
// emulated access that overlaps it can wait first.
//
// Each stamp is the timeline value covering the last RDP access (0 if never
// touched). Callers pass the stamp when marking and compare the result of
// conflict() against the GPU's completed timeline.
class RdramTracker {
public:
  enum class Access { Read, Write };

  // forget all stamps, e.g. when the timeline starts over; rdram_size fixes
  // the trackable range
  void reset(size_t rdram_size, size_t readback_page_size = 0) {
    readback_shift =
        readback_page_size ? std::countr_zero(readback_page_size) - 3 : 0;
    write_stamps.assign(rdram_size >> (3 + readback_shift), 0);
    read_stamps.assign(rdram_size >> 3, 0);
    block_write_stamps.assign(readback_shift ? read_stamps.size() : 0, 0);
    cleared_write_stamps.assign(readback_shift ? write_stamps.size() : 0, 0);
    newest = 0;
  }

  // A direct upload will clear this page's write mask before later draws.
  // Keep marks for draws that may still be queued in the renderer.
  void retire_readback_writes(size_t page, uint64_t completed) {
    cleared_write_stamps[page] = uint32_t(completed);
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
    // Timeline values stay far below 2^32 in practice (about one per SyncFull).
    uint32_t s = uint32_t(stamp);
    newest = std::max(newest, s);
    if (access == Access::Write && readback_shift) {
      std::fill(block_write_stamps.begin() + first,
                block_write_stamps.begin() + last + 1, s);
      std::fill(write_stamps.begin() + (first >> readback_shift),
                write_stamps.begin() + (last >> readback_shift) + 1, s);
      return;
    }
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
    uint32_t stamp = write_conflict(first, last);
    if (write)
      stamp = std::max(stamp, *std::max_element(read_stamps.begin() + first,
                                               read_stamps.begin() + last + 1));
    return stamp;
  }

private:
  // the 8-byte blocks [first, last] covering [address, address + length)
  bool block_range(uint64_t address, uint64_t length, size_t &first,
                   size_t &last) const {
    uint64_t blocks = read_stamps.size();
    first = address >> 3;
    if (length == 0 || first >= blocks)
      return false;
    last = std::min((address + length - 1) >> 3, blocks - 1);
    return true;
  }

  uint32_t write_conflict(size_t first, size_t last) const {
    if (!readback_shift)
      return *std::max_element(write_stamps.begin() + first,
                               write_stamps.begin() + last + 1);

    // A readback can replay earlier writes on the page until a direct upload
    // clears its write mask.
    uint32_t stamp = 0;
    while (first <= last) {
      size_t page = first >> readback_shift;
      size_t end = std::min(last + 1, (page + 1) << readback_shift);
      uint32_t page_stamp = write_stamps[page];
      if (page_stamp > stamp) {
        uint32_t cleared = cleared_write_stamps[page];
        if (std::any_of(block_write_stamps.begin() + first,
                        block_write_stamps.begin() + end,
                        [cleared](uint32_t block) { return block > cleared; }))
          stamp = page_stamp;
      }
      first = end;
    }
    return stamp;
  }

  std::vector<uint32_t> write_stamps;  // blocks for direct writes, pages for readback
  std::vector<uint32_t> read_stamps;   // texture loads, depth compare
  std::vector<uint32_t> block_write_stamps;   // possible GPU writes within pages
  std::vector<uint32_t> cleared_write_stamps; // retired write stamps per page
  unsigned readback_shift = 0;               // page shift relative to 8-byte blocks
  uint32_t newest = 0;
};
