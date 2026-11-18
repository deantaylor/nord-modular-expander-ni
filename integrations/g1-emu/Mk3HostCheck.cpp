// Headless integration check using the real G1-Emu/CoreAudio host, not a mock synth.
// GPL-3.0-or-later. Run only with an isolated flash path, never the user's bank file.
#include "Mk3ControlBridge.h"
#include <cstdio>
#include <cstdlib>
#include <atomic>
#include <csignal>
#include <thread>
#include <chrono>
static std::atomic<bool> stopped{false};
static void stopCheck(int){stopped.store(true);}
int main(int argc,char** argv) {
    if(argc!=6){std::fprintf(stderr,"usage: g1mk3check ROM ISOLATED_FLASH PATCH PORT SECONDS\n");return 2;}
    std::signal(SIGINT,stopCheck);std::signal(SIGTERM,stopCheck);
    const juce::ScopedJuceInitialiser_GUI gui;
    g1app::EmuHost host;host.options().gainDb=0.0f;
    std::string log;
    if(!host.start(argv[1],argv[2],log)){std::fprintf(stderr,"%s\n",log.c_str());return 1;}
    std::printf("%s\n",log.c_str());std::fflush(stdout);
    int result=0;
    {
        g1app::Mk3ControlBridge bridge(host,std::atoi(argv[4]));
        const auto upload=g1gui::preparePch(juce::File(juce::String(argv[3])),0);
        if(!upload.ok()){std::fprintf(stderr,"%s\n",upload.error.toRawUTF8());host.stop();return 1;}
        const double end=juce::Time::getMillisecondCounterHiRes()+std::atof(argv[5])*1000.;
        double nextReport=0;bool uploaded=false,queued=false,uiStalled=false;
        const int uiStallMs=std::getenv("G1_CHECK_UI_STALL_MS")?std::clamp(std::atoi(std::getenv("G1_CHECK_UI_STALL_MS")),0,1000):0;
        double uiStallAt=0;
        while(!stopped.load() && juce::Time::getMillisecondCounterHiRes()<end) {
            juce::MessageManager::getInstance()->runDispatchLoopUntil(20);
            const auto now=juce::Time::getMillisecondCounterHiRes();
            g1app::SynthSettings settings;uint64_t revision=0;
            if(!queued && host.synthSettings().settings(settings,revision)) {
                host.presets().upload(0,upload.frames,upload.abort,0,0);queued=true; // isolated bank 1/position 1
            }
            if(now>=nextReport){nextReport=now+1000.;const auto stats=host.stats();const auto status=host.presets().uploadStatus();
                std::printf("host: audio=%s peak=%.8f upload=%s busy=%d\n",stats.audio.c_str(),static_cast<double>(stats.peak),status.message.c_str(),status.busy?1:0);std::fflush(stdout);
                if(status.ok && !uploaded){for(int line=0;line<2;++line){std::printf("lcd raw %d:",line);for(unsigned char c:host.mc().getLcd().line(line))std::printf(" %02x",c);std::puts("");}}
                uploaded=uploaded||status.ok;
            }
            if(uploaded && uiStallMs>0 && !uiStalled) {
                if(uiStallAt==0)uiStallAt=now+2500.;
                if(now>=uiStallAt) {
                    uiStalled=true;std::printf("UI stall begin %dms\n",uiStallMs);std::fflush(stdout);
                    std::this_thread::sleep_for(std::chrono::milliseconds(uiStallMs));
                    std::puts("UI stall end");std::fflush(stdout);
                }
            }
        }
        if(!uploaded){std::fprintf(stderr,"Patch upload was not confirmed\n");result=1;}
    }
    host.stop();return result;
}
