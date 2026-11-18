#pragma once
#include <juce_core/juce_core.h>
#include <iostream>
// Private local metadata generated from the user's verified OS; no ROM access in the VST.
class ParameterNames {
public:
    ParameterNames(){
        const char* path=std::getenv("MK3_PARAMETER_NAMES");if(!path)return;
        const juce::File file(juce::String::fromUTF8(path));
        if(!file.existsAsFile() || file.getSize()>131072)return;
        load(juce::JSON::parse(file));
        if(names.isObject())std::cerr<<"Exact OS parameter labels loaded: "<<names.getDynamicObject()->getProperties().size()<<std::endl;
    }
    void load(const juce::var& data){
        names=juce::var();if(int(data["version"])!=1 || data["romSha256"].toString()!="d9b199f27f573fdf7cd985fbd33cb9e60893a08912ab5d1bb562c77c663c997e" || !data["names"].isObject())return;names=data["names"];
    }
    juce::String name(int type,int index,int maximum,const juce::String& fallback) const {
        if(type<0||type>=128||index<0||index>=128)return fallback;
        auto value=names[juce::Identifier(juce::String(type)+":"+juce::String(index))];auto text=value["name"].toString();
        if(!value["max"].isInt()||int(value["max"])!=maximum||text.isEmpty()||text.length()>11)return fallback;
        for(auto ch:text)if(ch<32||ch>=127)return fallback;
        return text;
    }
private:
    juce::var names;
};
