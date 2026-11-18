"""Test-only loopback relay: withhold PatchPacket replies, observe actual writes."""
import select
import socket
import threading
import time

class FaultLink:
    def __init__(self, target):
        self.target=target;self.drop_reads=False;self.writes=[];self.requests=[];self.dropped=0;self.error=None
        self.stop=threading.Event();self.listener=socket.socket();self.listener.bind(('127.0.0.1',0));self.listener.listen(1);self.listener.settimeout(.1)
        self.port=self.listener.getsockname()[1];self.thread=threading.Thread(target=self.run,daemon=True);self.thread.start()
    def run(self):
        peers=[]
        try:
            while not self.stop.is_set():
                try:client,_=self.listener.accept();break
                except socket.timeout:continue
            else:return
            upstream=socket.create_connection(('127.0.0.1',self.target),1);peers=[client,upstream]
            for p in peers:p.setsockopt(socket.IPPROTO_TCP,socket.TCP_NODELAY,1)
            buffers={p:bytearray() for p in peers}
            while not self.stop.is_set():
                ready,_,_=select.select(peers,[],[],.02)
                for src in ready:
                    data=src.recv(65536)
                    if not data:return
                    buf=buffers[src];buf.extend(data);dst=upstream if src is client else client
                    while buf:
                        if buf[0]!=0xf0:
                            # The official DirectLink greeting is the only text.
                            end=buf.find(b'\n')
                            if end<0:break
                            dst.sendall(bytes(buf[:end+1]));del buf[:end+1];continue
                        end=buf.find(b'\xf7')
                        if end<0:break
                        frame=bytes(buf[:end+1]);del buf[:end+1]
                        cc=frame[2]>>2 if len(frame)>3 else -1
                        if src is client and cc==0x17:self.requests.append(time.monotonic())
                        if src is client and cc==0x13 and len(frame)>9 and frame[5]==0x40:
                            self.writes.append((time.monotonic(),frame[9]))
                        if src is upstream and self.drop_reads and 0x1c<=cc<=0x1f:
                            self.dropped+=1;continue
                        dst.sendall(frame)
        except OSError as e:
            if not self.stop.is_set():self.error=str(e)
        finally:
            for p in peers:p.close()
    def close(self):
        self.stop.set();self.thread.join(timeout=2);self.listener.close()
