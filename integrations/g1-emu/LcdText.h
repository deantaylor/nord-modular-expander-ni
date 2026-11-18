// GPL-3.0-or-later. G1 rack OS 3.03 LCD text to UTF-8.
#pragma once
#include <string>
namespace g1app {
inline std::string lcdText(const std::string& raw) {
    std::string text;
    for(unsigned char c:raw) {
        // Verified with a real-ROM patch named gjpqy: 0A 0E 0B 0C 0D.
        // HD44780 CGRAM slots 0..7 are mirrored at 8..15.
        if(c<16) {
            switch(c&7) {
                case 2:text+='g';break;case 3:text+='p';break;case 4:text+='q';break;
                case 5:text+='y';break;case 6:text+='j';break;
                default:text+="�";break;
            }
        } else if(c==0x7e)text+="→";
        else if(c==0x7f)text+="←";
        else if(c>=32 && c<127)text+=static_cast<char>(c);
        else text+="�";
    }
    return text;
}
}
