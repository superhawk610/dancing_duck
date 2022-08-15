#include <SPI.h>
#include <Wire.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>
#include "image_bytes.h"

#define SCREEN_WIDTH 128
#define SCREEN_HEIGHT 64

#define OLED_RESET -1
#define SCREEN_ADDRESS 0x3c

Adafruit_SSD1306 display(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);

void setup() {
  Serial.begin(9600);

  // initialize display using internal 3.3V
  if (!display.begin(SSD1306_SWITCHCAPVCC, SCREEN_ADDRESS)) {
    Serial.println(F("SSD1306 allocation failed"));
    for(;;) {} // loop forever
  }

  // show splash screen
  display.display();
  delay(1000);

  // (optional) invert the display
  // display.invertDisplay(true);
}

uint32_t offset = 0;
void loop() {
  // display frames
  uint8_t* image_bytes = &image_frames[offset];
  int16_t w = pgm_read_byte(&image_bytes[0]);
  int16_t h = pgm_read_byte(&image_bytes[1]);

  // runtime decompression turns out to be an untenable approach since we
  // don't reliably have access to enough dynamic memory to compress all
  // of the raw pixel data (something as small as malloc(512) routinely
  // fails on memory-constrained boards)
  /* uint32_t compressed_len = 0; */
  /* for (int byte_index = 0; byte_index < 4; byte_index++) { */
  /*   compressed_len |= pgm_read_byte(&image_bytes[2 + byte_index]) << ((3 - byte_index) * 8); */
  /* } */

  /* uint32_t uncompressed_len = (uint32_t)((w + 7) / 8) * (uint32_t)h; */
  /* uint8_t* img = (uint8_t*)calloc(uncompressed_len, sizeof(uint8_t)); */

  /* uint32_t byte_index = 0; */
  /* uint8_t bit_index = 0; */
  /* for (uint32_t i = 0; i < compressed_len; i++) { */
  /*   uint8_t run_ = pgm_read_byte(&image_bytes[6 + i]); */
  /*   uint8_t bit_ = (run_ & (1 << 7)) > 0 ? 1 : 0; */
  /*   uint8_t len = (run_ & 0b01111111) + 1; */

  /*   for (uint8_t i = 0; i < len; i++) { */
  /*     if (bit_ == 1) img[byte_index] |= 1 << (7 - bit_index); */
  /*     bit_index += 1; */
  /*     if (bit_index == 8) { */
  /*       bit_index = 0; */
  /*       byte_index += 1; */
  /*     } */
  /*   } */
  /* } */

  displayImage(w, h, (const uint8_t[])&image_bytes[2]);
  delay(30);
  /* free(img); */

  offset += (uint32_t)((w + 7) / 8) * (uint32_t)h + 2;
  if (offset == (sizeof(image_frames) / sizeof(*image_frames))) {
    offset = 0;
  }
}

void displayImage(int16_t w, int16_t h, const uint8_t img[]) {
    display.clearDisplay();
    display.drawBitmap(
      (display.width() - w) / 2,
      (display.height() - h) / 2,
      img, w, h, 1
    );
    display.display();
}
