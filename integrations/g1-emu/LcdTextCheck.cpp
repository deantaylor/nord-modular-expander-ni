// GPL-3.0-or-later. Never lose G1 custom descender characters in names.
#include "LcdText.h"
#include <cstdio>
int main() {
    if(g1app::lcdText(std::string("\x0a\x0e\x0b\x0c\x0d",5))!="gjpqy")return 1;
    if(g1app::lcdText("MK3-Brid\x0a" "e")!="MK3-Bridge")return 2;
    if(g1app::lcdText("Em\x0b" "t\x0d \x0b" "atch")!="Empty patch")return 3;
    if(g1app::lcdText(std::string("\x02\x06\x03\x04\x05",5))!="gjpqy")return 4;
    if(g1app::lcdText("ABC 012! \x7e\x7f")!="ABC 012! →←")return 5;
    std::puts("PASS: G1 LCD descenders, mirrored CGRAM codes, normal text and arrows");return 0;
}
