// Regression: an acknowledged settings read must not hide the next client's ACK.
// GPL-3.0-or-later.
#include "synthsettings.h"
#include "pcsysex.h"
#include <cstdio>
int main() {
    using namespace g1app;
    using namespace g1app::pcsysex;
    SynthSettingsLink link;
    std::vector<uint8_t> outgoing, editor;
    link.tick(2500,outgoing);
    if(outgoing.empty())return 1;
    SynthSettings settings;
    auto payload=pack7(settings.encode());payload.insert(payload.begin(),0);
    link.g1Sent(frame(0x1e,0,payload),2516,editor);
    uint64_t revision=0;
    if(!editor.empty() || !link.settings(settings,revision))return 2;
    const auto ack=frame(0x16,0,{1,0x36,1});
    link.g1Sent(ack,2686,editor);
    if(editor!=ack){std::fprintf(stderr,"FAIL: completed settings read swallowed next upload ACK\n");return 3;}
    // A timed-out read still suppresses its late response.
    SynthSettingsLink timeout;outgoing.clear();editor.clear();
    timeout.tick(2500,outgoing);timeout.tick(4001,outgoing);
    timeout.g1Sent(frame(0x1e,0,payload),4050,editor);
    if(!editor.empty())return 4;
    std::puts("PASS: completed settings reply releases ACK ownership; abandoned reply stays filtered");
    return 0;
}
