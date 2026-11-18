cmake_minimum_required(VERSION 3.22)
project(G1KeyboardAdapter VERSION 0.1.0 LANGUAGES C CXX OBJC OBJCXX)
set(CMAKE_CXX_STANDARD 17)
if(NOT NME_SOURCE OR NOT JUCE_SOURCE)
  message(FATAL_ERROR "Pass NME_SOURCE and JUCE_SOURCE for the pinned upstream sources")
endif()
add_subdirectory("${JUCE_SOURCE}" juce)
set(S "${NME_SOURCE}/Source")
juce_add_console_app(g1keyboardadapter PRODUCT_NAME "G1 Keyboard Adapter")
file(GLOB PROTOCOL_SOURCES "${S}/protocol/*.cpp")
target_sources(g1keyboardadapter PRIVATE Adapter.cpp "${S}/format/ValueFormatters.cpp"
  "${S}/midi/ConnectionManager.cpp" "${S}/midi/MidiDeviceManager.cpp" "${S}/midi/DirectLink.cpp"
  "${S}/midi/NmProtocol.cpp" "${S}/midi/NmMessages.cpp" "${S}/midi/SysExCodec.cpp"
  "${S}/midi/MidiMonitor.cpp" "${S}/midi/OsUpdate.cpp" "${S}/midi/UploadPacketizer.cpp"
  "${S}/model/ThemeData.cpp" "${S}/model/LightMeterLayout.cpp"
  "${S}/model/Patch.cpp" "${S}/model/PatchParser.cpp" "${S}/model/PatchSerializer.cpp"
  "${S}/model/PchFileIO.cpp" "${S}/model/ModuleDescriptions.cpp" "${S}/model/ModuleTags.cpp"
  ${PROTOCOL_SOURCES})
juce_add_binary_data(KeyboardNmeData HEADER_NAME KeyboardNmeData.h NAMESPACE KeyboardNmeData SOURCES "${NME_SOURCE}/data/modules.xml" "${NME_SOURCE}/data/classic-theme.xml")
target_include_directories(g1keyboardadapter PRIVATE "${S}")
target_compile_definitions(g1keyboardadapter PRIVATE JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0 JUCE_MODAL_LOOPS_PERMITTED=1)
target_link_libraries(g1keyboardadapter PRIVATE KeyboardNmeData juce::juce_gui_basics juce::juce_audio_devices)

# Hardware-free callback checks; no synth, audio, or MIDI device is opened.
juce_add_console_app(g1midiactivitycheck PRODUCT_NAME "G1 MIDI Activity Check")
target_sources(g1midiactivitycheck PRIVATE MidiActivityCheck.cpp)
target_compile_definitions(g1midiactivitycheck PRIVATE JUCE_WEB_BROWSER=0 JUCE_USE_CURL=0)
target_link_libraries(g1midiactivitycheck PRIVATE juce::juce_gui_basics juce::juce_audio_devices)
