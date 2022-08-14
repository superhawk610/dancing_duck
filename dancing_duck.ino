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

int32_t offset = 0;
void loop() {
  // display frames
  // TODO: compress empty pixels to reduce PROGMEM usage and allow storing larger animations
  uint8_t* image_bytes = &image_frames[offset];
  int16_t w = pgm_read_byte(&image_bytes[0]);
  int16_t h = pgm_read_byte(&image_bytes[1]);
  displayImage(w, h, (const uint8_t[])&image_bytes[2]);
  delay(30);

  offset += (int32_t)((w + 7) / 8) * (int32_t)h + 2;
  if (offset == (sizeof image_frames / sizeof *image_frames)) {
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
