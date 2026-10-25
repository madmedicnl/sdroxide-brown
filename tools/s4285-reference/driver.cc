// A standalone driver for the reference STANAG 4285 decoder in this repo.
//
// The upstream project drives `s4285::bitstream_decoder` from Octave
// (`src/s4285_cc.cc`) with `d.process(z(i))` per complex sample of the analytic
// signal at 12 kHz. Nothing here changes the decoder — this only replaces the
// Octave glue, so the same bits come out without Octave installed.
//
//   ffmpeg -i capture.wav -ac 1 -ar 12000 -f f32le /tmp/opencode/cap.raw
//   ./driver /tmp/opencode/cap.raw 2 > bits.txt
//
// argv[1] = raw mono f32le at 12 kHz, argv[2] = modulation bits per symbol.

#include <cstdio>
#include <cstdlib>
#include <cmath>
#include <complex>
#include <vector>

#include "s4285_bitstream_decoder.hpp"

// 101-tap windowed Hilbert transformer: real in, analytic out.
class analytic {
public:
  explicit analytic(int taps = 101) : _h(taps, 0.0f), _x(taps, 0.0f), _i(0) {
    const int m = taps / 2;
    for (int n = 0; n < taps; ++n) {
      const int k = n - m;
      double h = 0.0;
      if (k & 1) h = 2.0 / (M_PI * k);
      // Blackman window
      const double w = 0.42 - 0.5 * cos(2 * M_PI * n / (taps - 1))
                     + 0.08 * cos(4 * M_PI * n / (taps - 1));
      _h[n] = float(h * w);
    }
  }
  std::complex<float> push(float s) {
    _x[_i] = s;
    float acc = 0.0f;
    int idx = _i;
    for (int n = 0; n < int(_h.size()); ++n) {
      acc += _h[n] * _x[idx];
      idx = (idx == 0 ? int(_x.size()) - 1 : idx - 1);
    }
    _i = (_i + 1 == int(_x.size()) ? 0 : _i + 1);
    return std::complex<float>(s, acc);
  }
private:
  std::vector<float> _h, _x;
  int _i;
};

class collector : public s4285::bitstream_decoder {
public:
  collector(int taps_data, int taps_symbols, int mod)
    : s4285::bitstream_decoder(taps_data, taps_symbols, mod), chunks(0) {}
  std::vector<int> bits;
  std::vector<unsigned char> soft;   // 128 per frame, the decoder's own input
  int chunks;
protected:
  virtual void process_frame_data(uint8_vector_type::const_iterator i0,
                                  uint8_vector_type::const_iterator i1) {
    for (int j = 0; j < 128; ++j) soft.push_back((unsigned char)soft_decision(j));
    s4285::bitstream_decoder::process_frame_data(i0, i1);
  }
  virtual void process_bits(std::vector<int>::const_iterator i0,
                            std::vector<int>::const_iterator i1) {
    bits.insert(bits.end(), i0, i1);
    chunks += 1;
  }
};

int main(int argc, char** argv) {
  if (argc < 3) { fprintf(stderr, "usage: %s raw.f32le mod\n", argv[0]); return 2; }
  const int mod = atoi(argv[2]);
  FILE* f = fopen(argv[1], "rb");
  if (!f) { perror("open"); return 1; }
  std::vector<float> audio;
  float buf[4096];
  size_t n;
  while ((n = fread(buf, sizeof(float), 4096, f)) > 0) audio.insert(audio.end(), buf, buf + n);
  fclose(f);
  fprintf(stderr, "samples %zu\n", audio.size());

  // The reference works on **baseband**: its PLL tracks a fractional
  // symbol-rate error and a few Hz, not the 1800 Hz the audio carries.
  const double carrier = (argc >= 4 ? atof(argv[3]) : 1800.0);
  const double inc = -2.0 * M_PI * carrier / 12000.0;
  analytic an;
  collector d(12, 4, mod);
  for (size_t i = 0; i < audio.size(); ++i) {
    const std::complex<float> z = an.push(audio[i]);
    const double ph = inc * double(i);
    d.process(std::complex<float>(float(z.real() * cos(ph) - z.imag() * sin(ph)),
                                  float(z.real() * sin(ph) + z.imag() * cos(ph))));
  }

  fprintf(stderr, "chunks %d bits %zu frames %zu\n", d.chunks, d.bits.size(), d.soft.size()/128);
  if (FILE* sf = fopen("/tmp/opencode/ref_soft.bin", "wb")) {
    fwrite(d.soft.data(), 1, d.soft.size(), sf);
    fclose(sf);
  }
  if (FILE* bf = fopen("/tmp/opencode/ref_bits_clean.txt", "w")) {
    for (size_t i = 0; i < d.bits.size(); ++i) fputc(d.bits[i] ? '1' : '0', bf);
    fclose(bf);
  }
  for (size_t i = 0; i < d.bits.size(); ++i) putchar(d.bits[i] ? '1' : '0');
  putchar('\n');
  return 0;
}
