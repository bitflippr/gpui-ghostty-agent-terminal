#include <array>
#include <span>
#include <string>
#include <vector>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <immintrin.h>
#include <intrin.h>
#include <windows.h>
#include <chrono>
#include <fstream>
#include <algorithm>
#define DRAC_ARCH_X86_64 1
using u8 = uint8_t; using u32 = uint32_t; using usize = size_t; using String = std::string;
template <class T, size_t N> using Array = std::array<T, N>;
template <class T> using Span = std::span<T>;    constexpr Array<char, 65> BASE64_TABLE = {
      'A',
      'B',
      'C',
      'D',
      'E',
      'F',
      'G',
      'H',
      'I',
      'J',
      'K',
      'L',
      'M',
      'N',
      'O',
      'P',
      'Q',
      'R',
      'S',
      'T',
      'U',
      'V',
      'W',
      'X',
      'Y',
      'Z',
      'a',
      'b',
      'c',
      'd',
      'e',
      'f',
      'g',
      'h',
      'i',
      'j',
      'k',
      'l',
      'm',
      'n',
      'o',
      'p',
      'q',
      'r',
      's',
      't',
      'u',
      'v',
      'w',
      'x',
      'y',
      'z',
      '0',
      '1',
      '2',
      '3',
      '4',
      '5',
      '6',
      '7',
      '8',
      '9',
      '+',
      '/',
      '\0'
    };

#if defined(_WIN32) && DRAC_ARCH_X86_64 && defined(__clang__)
    __attribute__((target("xsave"))) auto HasAvxState() -> bool {
      return (_xgetbv(0) & 6) == 6;
    }

    auto HasAvx2() -> bool {
      int registers[4];
      __cpuid(registers, 0);
      if (registers[0] < 7)
        return false;
      __cpuid(registers, 1);
      if ((registers[2] & 0x18000000) != 0x18000000 || !HasAvxState())
        return false;
      __cpuidex(registers, 7, 0);
      return (registers[1] & 0x20) != 0;
    }

    __attribute__((target("avx2"))) auto Base64EncodeAvx2(const u8* data, usize size, char* out) -> usize {
      const auto shuffle = _mm256_setr_epi8(1, 0, 2, 1, 4, 3, 5, 4, 7, 6, 8, 7, 10, 9, 11, 10,
                                            1, 0, 2, 1, 4, 3, 5, 4, 7, 6, 8, 7, 10, 9, 11, 10);
      const auto offsets = _mm256_setr_epi8(65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -19, -16, 0, 0,
                                            65, 71, -4, -4, -4, -4, -4, -4, -4, -4, -4, -4, -19, -16, 0, 0);
      usize consumed = 0;
      // Each lane consumes 12 bytes. The extra four loaded bytes stay inside the input.
      while (size - consumed >= 28) {
        auto input = _mm256_castsi128_si256(_mm_loadu_si128(reinterpret_cast<const __m128i*>(data + consumed)));
        input = _mm256_inserti128_si256(input, _mm_loadu_si128(reinterpret_cast<const __m128i*>(data + consumed + 12)), 1);
        input = _mm256_shuffle_epi8(input, shuffle);
        const auto high = _mm256_mulhi_epu16(_mm256_and_si256(input, _mm256_set1_epi32(0x0fc0fc00)), _mm256_set1_epi32(0x04000040));
        const auto low = _mm256_mullo_epi16(_mm256_and_si256(input, _mm256_set1_epi32(0x003f03f0)), _mm256_set1_epi32(0x01000010));
        const auto sextets = _mm256_or_si256(high, low);
        const auto indices = _mm256_sub_epi8(_mm256_subs_epu8(sextets, _mm256_set1_epi8(51)), _mm256_cmpgt_epi8(sextets, _mm256_set1_epi8(25)));
        const auto encoded = _mm256_add_epi8(sextets, _mm256_shuffle_epi8(offsets, indices));
        _mm256_storeu_si256(reinterpret_cast<__m256i*>(out + consumed / 3 * 4), encoded);
        consumed += 24;
      }
      return consumed;
    }
#endif

    auto Base64EncodeInto(Span<const u8> data, char* out) -> void {
      usize idx = 0;
#if defined(_WIN32) && DRAC_ARCH_X86_64 && defined(__clang__)
      static const bool useAvx2 = HasAvx2();
      if (useAvx2)
        idx = Base64EncodeAvx2(data.data(), data.size(), out);
#endif
      usize dest = idx / 3 * 4;
      while (data.size() - idx >= 3) {
        const u32 triple = (static_cast<u32>(data[idx]) << 16) |
          (static_cast<u32>(data[idx + 1]) << 8) | static_cast<u32>(data[idx + 2]);
        out[dest++] = BASE64_TABLE[(triple >> 18) & 0x3f];
        out[dest++] = BASE64_TABLE[(triple >> 12) & 0x3f];
        out[dest++] = BASE64_TABLE[(triple >> 6) & 0x3f];
        out[dest++] = BASE64_TABLE[triple & 0x3f];
        idx += 3;
      }
      if (idx < data.size()) {
        const bool twoBytes = data.size() - idx == 2;
        const u32 triple = (static_cast<u32>(data[idx]) << 16) |
          (twoBytes ? static_cast<u32>(data[idx + 1]) << 8 : 0);
        out[dest++] = BASE64_TABLE[(triple >> 18) & 0x3f];
        out[dest++] = BASE64_TABLE[(triple >> 12) & 0x3f];
        out[dest++] = twoBytes ? BASE64_TABLE[(triple >> 6) & 0x3f] : '=';
        out[dest] = '=';
      }
    }

    auto Base64Encode(Span<const u8> data) -> String {
      String out;
      out.resize_and_overwrite(((data.size() + 2) / 3) * 4, [&](char* dest, usize size) {
        Base64EncodeInto(data, dest);
        return size;
      });
      return out;
    }

std::string Reference(std::span<const uint8_t> data) {
  std::string output; uint32_t bits=0; int count=0;
  for(auto byte:data) { bits=(bits<<8)|byte; count+=8; while(count>=6){count-=6;output.push_back(BASE64_TABLE[(bits>>count)&63]);} }
  if(count) output.push_back(BASE64_TABLE[(bits<<(6-count))&63]);
  while(output.size()%4)output.push_back('='); return output;
}
std::string Old(std::span<const uint8_t> data) {
  std::string output; output.reserve((data.size()+2)/3*4); size_t i=0;
  for(;i+2<data.size();i+=3) { uint32_t bits=(uint32_t(data[i])<<16)|(uint32_t(data[i+1])<<8)|data[i+2]; output.push_back(BASE64_TABLE[(bits>>18)&63]);output.push_back(BASE64_TABLE[(bits>>12)&63]);output.push_back(BASE64_TABLE[(bits>>6)&63]);output.push_back(BASE64_TABLE[bits&63]); }
  if(i<data.size())output+=Reference(data.subspan(i));return output;
}
int main(int argc,char**argv) {
  std::vector<uint8_t> random(16384); uint32_t state=42;
  for(auto& byte:random) {state=state*1664525+1013904223;byte=state>>24;}
  for(size_t offset=0;offset<32;++offset) for(size_t size=0;size<4096;++size) {
    auto input=std::span<const uint8_t>(random).subspan(offset,size);
    auto expected=Reference(input); auto actual=Base64Encode(input);
    if(expected!=actual){printf("FAIL offset=%zu size=%zu\n",offset,size);return 1;}
  }
  auto* pages=static_cast<uint8_t*>(VirtualAlloc(nullptr,8192,MEM_RESERVE|MEM_COMMIT,PAGE_READWRITE)); DWORD old;
  if (!pages) { std::fprintf(stderr,"Guard-page allocation failed: %lu\n",GetLastError()); return 4; }
  if (!VirtualProtect(pages+4096,4096,PAGE_NOACCESS,&old)) {
    const DWORD error=GetLastError(); VirtualFree(pages,0,MEM_RELEASE);
    std::fprintf(stderr,"Guard-page protection failed: %lu\n",error); return 4;
  }
  for(size_t size=0;size<1024;++size) {
    auto* start=pages+4096-size; std::memcpy(start,random.data(),size);
    auto actual=Base64Encode(std::span<const uint8_t>(start,size));
    if(actual!=Reference(std::span<const uint8_t>(start,size)))return 2;
  }
  VirtualFree(pages,0,MEM_RELEASE);
  printf("PASS 131072 random length/alignment cases, 1024 guard-page tails, AVX2=%d\n",HasAvx2());
  if(argc<2)return 0;
  std::ifstream file(argv[1],std::ios::binary|std::ios::ate);
  if (!file) { std::fprintf(stderr,"Cannot open benchmark input: %s\n",argv[1]); return 4; }
  const auto n=file.tellg();
  if (n<=0) { std::fprintf(stderr,"Benchmark input is empty or unreadable\n"); return 4; }
  std::vector<uint8_t> bytes(static_cast<size_t>(n)); file.seekg(0);
  if (!file.read(reinterpret_cast<char*>(bytes.data()),n)) { std::fprintf(stderr,"Could not read the complete benchmark input\n"); return 4; }
  auto expected=Old(bytes); std::vector<double> oldTimes,newTimes;
  for(int i=0;i<12;++i) {
    auto a=std::chrono::steady_clock::now(); auto oldOutput=Old(bytes);auto b=std::chrono::steady_clock::now();
    auto newOutput=Base64Encode(bytes);auto c=std::chrono::steady_clock::now();
    if(newOutput!=expected || oldOutput!=expected)return 3;
    oldTimes.push_back(std::chrono::duration<double,std::milli>(b-a).count());newTimes.push_back(std::chrono::duration<double,std::milli>(c-b).count());
  }
  std::sort(oldTimes.begin(),oldTimes.end());std::sort(newTimes.begin(),newTimes.end());printf("bytes=%zu old_median_ms=%.3f new_median_ms=%.3f\n",bytes.size(),(oldTimes[5]+oldTimes[6])/2,(newTimes[5]+newTimes[6])/2);
}
