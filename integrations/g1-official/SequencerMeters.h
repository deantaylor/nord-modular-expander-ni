#pragma once
#include <array>
#include <cstdint>
#include <optional>

// Values come from NMInfo MeterMessage pairs, never from an estimated clock.
class SequencerMeters {
public:
    void reset(){slot=-1;generation=0;times.fill(-1);values.fill(16);}
    void ingest(int selected,uint64_t epoch,int startPair,const int* b,const int* a,int count,int64_t now){
        if(startPair<0||startPair>=64||count<1||count>5)return;
        if(slot!=selected||generation!=epoch){reset();slot=selected;generation=epoch;}
        for(int i=0;i<count&&startPair+i<64;++i){const int pair=startPair+i;values[pair*2]=b[i];values[pair*2+1]=a[i];times[pair]=now;}
    }
    std::optional<int> step(int selected,uint64_t epoch,int base,int64_t now) const {
        if(slot!=selected||generation!=epoch||base<0||base>=128||times[base/2]<0)return {};
        const auto age=now-times[base/2];if(age<0||age>2000)return {};
        const int value=values[base];return value>=0&&value<16?std::optional<int>(value):std::nullopt;
    }
private:
    int slot=-1;uint64_t generation=0;std::array<int,128> values{};
    std::array<int64_t,64> times=[](){std::array<int64_t,64> result{};result.fill(-1);return result;}();
};
