import threading
import numpy as np
#import queue as Queue
import time
import uhd # type: ignore
import multiprocessing as mp
import multiprocessing.shared_memory as shared_memory
import multiprocessing.queues as mp_queues
import sys
import threading
import os
import queue

from time import sleep

############### START LORA_TRANSCEIVER.PY #########################

class bcolors:
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


PRINT_DEBUG = False

np.set_printoptions(threshold=sys.maxsize)

def threshold_trigger_process(index_queue,SF_sample_th, queue_arr_tuple):
    queue_arr = np.array(queue_arr_tuple, dtype= mp_queues.Queue)
    samples_counter = np.zeros((SF_sample_th.size))

    while True:

        item = index_queue.get()
        samples_counter = samples_counter + 1
        for index, th in enumerate(SF_sample_th):
            if samples_counter[index] >= th:
                queue_arr[index].put_nowait(1)
                samples_counter[index] = 0


def decoder_process(shm_name,complex_data_number, buf_size,sf_index_queue,pkt_queue, sf, BW, fs, SF_sample_th):
    # temp_samples = np.zeros(complex_data_number,dtype=np.complex64)



    print("Decoder on SF", str(sf), ", PID:", str(os.getgid())) if PRINT_DEBUG else None
    sf_minimum_win_size_arr = np.array((650e3, 1.2e6, 2e6, 3.6e6, 8e6, 14.5e6), dtype= np.uint32)
    sf_minimum_win_size = int(1.1 * sf_minimum_win_size_arr[sf-7])

    buf_length = complex_data_number * buf_size
    temp_samples = np.zeros(sf_minimum_win_size, dtype=np.complex64)
    existing_shm = shared_memory.SharedMemory(name=shm_name)
    rec_buffer = np.ndarray((buf_length,), dtype=np.complex64, buffer=existing_shm.buf)
    sf_windows_len = complex_data_number * SF_sample_th
    old_rec = 0
    rep_count = 0

    win_start_index = 0

    count_debug = 0

    while True:

        count_debug = count_debug + 1
        if(count_debug == 50):
            print(bcolors.OKGREEN + "process on SF",sf, "is alive!!!" + bcolors.ENDC) if PRINT_DEBUG else None
            count_debug = 0
        #sys.stdout.flush()
        # start_t = time.time()
        #print("Decoder")
        item = sf_index_queue.get()
        #print("Starting decoding on SF",sf)
        c = rec_buffer[win_start_index:win_start_index + sf_windows_len]
        win_start_index = (win_start_index + sf_windows_len) % buf_length
        # print(c.tolist())
        # c.real = np.array(samples_buffer[cursor:cursor+block_size:2 * (1 * float_size) ], dtype = np.float32)
        # c.imag = np.array(samples_buffer[cursor+(1 * float_size) :cursor+block_size:2 * (1 * float_size) ], dtype = np.float32)
        #DECODE THE SAMPLES THROUGH THE DECODE FUNCTION FROM THE LORA MODULE
        #print("Decoder End")

        #print("Passing the samples from sf", sf,  "to a decoder process")
        (mp.Process(target=thread_decode, args=(np.concatenate((temp_samples,c)), sf, BW, fs, pkt_queue))).start()

        temp_samples = c[-sf_minimum_win_size:]

    return
    #print("esco")

def thread_decode(samples, sf, BW, fs, pkt_queue):
    #print(bcolors.OKBLUE + "[DECODER PROCESS] Decoding samples from sf", sf, bcolors.ENDC)
    ans = decode(samples, sf, BW, fs)
    if ans.size > 0:
        #print("# DECODED PACKETS", ans.size)
        for pkt in ans:
            pkt_queue.put_nowait(pkt)
    else:
        pkt_queue.put_nowait(int(0))

        # for pkt in ans:
        #     print(pkt)



def tx_burst(sample_rate, center_freq, pkt_list, sleep_time, sending, streamer, amplitude, verbose):

    #usrp = uhd.usrp.MultiUSRP("address=" + address)

    #print("TX Acquiring")
    metadata = uhd.types.TXMetadata()
    for index,pkt in enumerate(pkt_list):
        print("Transmitter: Processing Packet #", index) if PRINT_DEBUG else None

        sending.value = True
        buffer_samps = streamer.get_max_num_samps()

        samples = amplitude * encode(center_freq, pkt.SF, pkt.BW, pkt.payload, sample_rate, pkt.src, pkt.dst, pkt.seqn, 1, 1, 0, 8)
        print("Encoded Packet #", index) if PRINT_DEBUG else None
        proto_len = samples.shape[-1]
        send_samps = 0
        samples = samples.reshape(1, samples.size)
        while send_samps < proto_len:
            real_samps = min(proto_len, buffer_samps - send_samps)
            if real_samps < proto_len:
                n_samples = streamer.send(samples[:real_samps], metadata)
            else:
                n_samples = streamer.send(samples, metadata)
            send_samps += n_samples
            if (verbose):
                print("Sent samples", n_samples) if PRINT_DEBUG else None
        metadata.end_of_burst = True
        print("Ending Burst for Packet #", index) if PRINT_DEBUG else None

        streamer.send(np.zeros((1, 1), dtype=np.complex64), metadata)

        print("Sent Packet #", index) if PRINT_DEBUG else None

        # Send EOB to terminate Tx

        #print("Sent packet with seq number", pkt.seqn)
        time.sleep(sleep_time)

    sending.value = False


def tx_burst_multi_sf(sample_rate, center_freq, pkt_list, sleep_time, sending, streamer, sf_list, amplitude):

    #usrp = uhd.usrp.MultiUSRP("address=" + address)


    n_pack = len(pkt_list)
    print("N_pack:", n_pack)
    #remaining_pkt = np.array([len(pkt_list)] * len(sf_list))
    pkt_count = np.zeros((len(sf_list)-1), dtype=np.uint32)
    sf_list.sort(reverse = True)
    #print("TX Acquiring")
    metadata = uhd.types.TXMetadata()
    for index,pkt in enumerate(pkt_list):

        sending.value = True
        #print("TX Acquired")

        buffer_samps = streamer.get_max_num_samps()
        #print("LOL")



        samples = amplitude * encode(center_freq, sf_list[0], pkt.BW, pkt.payload, sample_rate, pkt.src, pkt.dst, pkt.seqn, 1, 1, 0, 8)
        max_len = samples.size

        for i,sf in enumerate(sf_list[1:]):
            add_samples = np.zeros((samples.size), dtype=np.complex64)
            length = 0
            incr = 0
            coded_pkt = 0
            while (pkt_count[i] < n_pack):
                current_pkt = pkt_list[pkt_count[i]]
                current_samples = encode(center_freq, sf, current_pkt.BW, current_pkt.payload, sample_rate, current_pkt.src, current_pkt.dst,
                                      current_pkt.seqn, 1, 1, 0, 8)


                add_samples[length:length + current_samples.size] = current_samples
                length = length + current_samples.size
                coded_pkt = coded_pkt + 1
                pkt_count[i] = pkt_count[i] + 1
                if (length * ((coded_pkt + 1) / coded_pkt)) > max_len:
                    print("Pkt Count for SF", sf,":", pkt_count[i]) if PRINT_DEBUG else None
                    break




            samples = samples + add_samples



        proto_len = samples.shape[-1]
        send_samps = 0
        samples = samples.reshape(1, samples.size)
        while send_samps < proto_len:

            real_samps = min(proto_len, buffer_samps - send_samps)
            if real_samps < proto_len:
                n_samples = streamer.send(samples[:real_samps], metadata)
            else:
                n_samples = streamer.send(samples, metadata)
            send_samps += n_samples

            metadata.end_of_burst = True
            streamer.send(np.zeros((1, 1), dtype=np.complex64), metadata)


        # Send EOB to terminate Tx

        #print("Sent packet with seq number", pkt.seqn)
        time.sleep(sleep_time)

    sending.value = False






def tx(sample_rate, center_freq, pkt_queue, sleep_time, sending, streamer, amplitude, verbose = False):

    #usrp = uhd.usrp.MultiUSRP("address=" + address)

    sending.value = True
    metadata = uhd.types.TXMetadata()
    while sending.value:

        pkt = pkt_queue.get()
        if(verbose):
            print("Sending pkt", pkt)

        buffer_samps = streamer.get_max_num_samps()

        samples = amplitude * encode(center_freq, pkt.SF, pkt.BW, pkt.payload, sample_rate, pkt.src, pkt.dst, pkt.seqn, 1, 1, 0, 8)
        proto_len = samples.shape[-1]
        send_samps = 0
        samples = samples.reshape(1, samples.size)
        while send_samps < proto_len:



            real_samps = min(proto_len, buffer_samps - send_samps)
            if real_samps < proto_len:
                n_samples = streamer.send(samples[:real_samps], metadata)
            else:
                n_samples = streamer.send(samples, metadata)
            send_samps += n_samples

            if(verbose):
                print("Sent samples", n_samples)
                print("Total Sent Samples", send_samps)

        metadata.end_of_burst = True
        streamer.send(np.zeros((1, 1), dtype=np.complex64), metadata)


        # Send EOB to terminate Tx

        #print("Sent packet with seq number", pkt.seqn)
        time.sleep(sleep_time)

    sending.value = False













def rx(sample_rate, sf_list, bandwidth, receiving, packet_queue, complex_data_number, streamer, pause_rec,buf_size = 120):






    #LORA RECEIVER MAIN THREAD#


    #THE RECEIVER SCRIPT IS STRUCTURED AS A TWO-PROCESS PROGRAM: ONE PROCESS (THE MAIN THREAD OR RECEIVER THREAD) IS RESPONSIBLE FOR READING AND
    #BUFFERING OF RF DATA FROM THE USRP RADIO; THE OTHER PROCESS (THE DECODER THREAD) READS AND PROCESSES DATA FROM THE BUFFER
    #THE PROGRAM RESORTS TO A CIRCULAR BUFFER, AND MAKES USE OF A QUEUE TO EXCHANGDE DATA BETWEEN THE PROCESSES
    #MORE IN DETAIL, THE BUFFER IS LOCATED IN A SHARED MEMORY AREA. EACH TIME A NEW CHUNK OF DATA IS RECEIVED, THE RECEIVER PUTS THE DATA START INDEX IN THE QUEUE.
    #THE PROCESSING THREAD CAN ACCORDINGLY READ AND, POSSIBLY, DECODE THE LORA DATA IN THE CHUNK.


    #MAXIMUM NUMBER OF DATA CHUNKS IN THE SHARED MEMORY BUFFER
    BUF_SIZE = int(buf_size) #MAKE SURE THIS NUMBER IS A MULTIPLE OF THE MAXIMUM SAMPLES THRESHOLD DIVIDED BY COMPLEX_DATA_NUMBER
    #FOR INSTANCE, WE NOW HAVE 72 MS FOR SF 12, AND 3M AS COMPLEX_DATA_NUMBER. 72/3 = 24, AND 120 IS INDEED A MULTIPLE OF 24

    #BYTES PER COMPLEX SAMPLES
    data_size = 8  # bytes

    #NUMBER OF COMPLEX SAMPLES IN A DATA CHUNK
    #complex_data_number = 500000



    #SLIDING WINDOW CURSOR FOR THE RECEIVER BUFFER
    cursor = 0




    #SIZE, IN BYTES, OF A DATA CHUNK
    block_size = complex_data_number * data_size
    #samples_buffer = np.zeros(, dtype=np.complex64)

    #PROCESS UTILITIES
    #complex_data_number
    #SF_sample_th = np.array([3e6,6e6,12e6,18e6,42e6,72e6])


    #NUMBER OF RECEIVING WINDOWS PER SF. 
    SF_sample_th = np.array([1, 1, 1, 1, 1, 1])
    sf_arr = np.array([7, 8, 9, 10, 11, 12])


    indexes = np.isin(sf_arr, sf_list)

    SF_sample_th = SF_sample_th[indexes]
    sf_arr = sf_arr[indexes]


    queue_arr = np.empty(shape=(sf_arr.size,), dtype=mp_queues.Queue)
    processes_arr = np.empty(shape=(sf_arr.size,), dtype=mp.Process)





    #CREATION OF THE SHARED MEMORY AREA
    shm = shared_memory.SharedMemory(create = True, size = block_size * BUF_SIZE)
    samples_buffer = np.ndarray(complex_data_number * BUF_SIZE, dtype=np.complex64, buffer=shm.buf)
    buffer_size = samples_buffer.size
    index_queue = mp.Queue(0)


    print("########################")
    print(mp.get_start_method('spawn'))
    print("########################")
    #CREATION OF THE DECODER PROCESSES

    for index, sf in enumerate(sf_arr):
        queue_arr[index] = mp.Queue(0)
        processes_arr[index] = mp.Process(name="Decoder SF" + str(sf), target=decoder_process, args=(shm.name, complex_data_number, BUF_SIZE,
                                                                        queue_arr[index], packet_queue[index], sf, bandwidth,
                                                                        sample_rate, SF_sample_th[index]))


        processes_arr[index].start()
        while (not (processes_arr[index].is_alive())):
            time.sleep(0.1)

    for index, sf in enumerate(sf_arr):
        while (not (processes_arr[index].is_alive())):
            processes_arr[index] = mp.Process(name="Decoder SF" + str(sf), target=decoder_process,
                                              args=(shm.name, complex_data_number, BUF_SIZE,
                                                    queue_arr[index], packet_queue[index], sf, bandwidth,
                                                    sample_rate, SF_sample_th[index]))

            processes_arr[index].start()
            time.sleep(1)


    #CREATION OF THE THRESHOLD TRIGGER PROCESS


    trigger_process = mp.Process(target=threshold_trigger_process, args=(index_queue, SF_sample_th, tuple(queue_arr)))
    trigger_process.daemon = True
    trigger_process.start()


    debug = True


    # # Start Stream
    stream_cmd = uhd.types.StreamCMD(uhd.types.StreamMode.start_cont)
    stream_cmd.stream_now = True
    streamer.issue_stream_cmd(stream_cmd)
    receiving.value = True

    metadata = uhd.types.RXMetadata()

    print("Starting...")

    #("Channels",streamer.get_num_channels())
    buf_length = complex_data_number
    recv_buffer = np.zeros((buf_length,), dtype=np.complex64)
    #START RECEIVING
    while receiving.value:
        try:

            pause_rec.wait()
            streamer.recv(recv_buffer, metadata)
            #streamer.recv(recv_buffer, metadata, timeout = 10)
            #streamer.recv(samples_buffer[cursor:cursor + complex_data_number], metadata)
            
            print(bcolors.OKGREEN + "DEBUG metadata ", metadata, bcolors.ENDC) if PRINT_DEBUG else None

            samples_buffer[cursor:cursor + buf_length] = recv_buffer
            cursor = (cursor + buf_length) % buffer_size
            if(cursor % complex_data_number == 0):
                index_queue.put_nowait(cursor)
            # cursor = (cursor + complex_data_number) % buffer_size
            #print("Rec")
            if(cursor == 0):
                # stream_cmd = uhd.types.StreamCMD(uhd.types.StreamMode.stop_cont)
                # streamer.issue_stream_cmd(stream_cmd)
                # time.sleep(0.1)
                # stream_cmd = uhd.types.StreamCMD(uhd.types.StreamMode.start_cont)
                # stream_cmd.stream_now = True
                # streamer.issue_stream_cmd(stream_cmd)
                pass




        except KeyboardInterrupt:
            print("Stopping RX")
            rec = False
            # Stop Stream
            stream_cmd = uhd.types.StreamCMD(uhd.types.StreamMode.stop_cont)
            streamer.issue_stream_cmd(stream_cmd)
            for proc in processes_arr:
                proc.terminate()
                proc.join()
            trigger_process.terminate()
            trigger_process.join()
            shm.close()
            shm.unlink()
            break

    else:
        print("Stopping RX")
        # Stop Stream
        stream_cmd = uhd.types.StreamCMD(uhd.types.StreamMode.stop_cont)
        streamer.issue_stream_cmd(stream_cmd)
        for proc in processes_arr:
            proc.terminate()
            proc.join()
        trigger_process.terminate()
        trigger_process.join()
        shm.close()
        shm.unlink()















class lora_transceiver():

    def __init__(self,address,rx_gain,tx_gain,bandwidth, rx_freq, tx_freq, sample_rate, rx_channel_ID, tx_channel_ID, signal_amplitude = 1):
        self.address = address
        self.rx_gain = rx_gain # dB
        self.tx_gain = tx_gain # dB
        self.bandwidth = bandwidth  # Hz
        self.rx_freq = rx_freq # Hz
        self.tx_freq = tx_freq  # Hz
        self.sample_rate = sample_rate # Hz
        self.receiving = mp.Value("i",False)
        self.sending = mp.Value("i",False)
        self.signal_amplitude = signal_amplitude
        self.rx_channel = rx_channel_ID
        self.tx_channel = tx_channel_ID
        self.usrp = uhd.usrp.MultiUSRP("address=" + self.address)
        self.usrp.set_rx_rate(sample_rate, self.rx_channel)
        self.usrp.set_rx_freq(uhd.libpyuhd.types.tune_request(rx_freq), self.rx_channel)
        self.usrp.set_rx_gain(rx_gain, self.rx_channel)
        self.usrp.set_rx_bandwidth(bandwidth, self.rx_channel)
        # Set up the stream and receive buffer
        st_args = uhd.usrp.StreamArgs("fc32", "sc16")
        st_args.channels = [self.rx_channel]
        self.rx_streamer = self.usrp.get_rx_stream(st_args)
        self.rx_pause_flag = mp.Event()
        self.rx_pause_flag.set()
        #

        self.usrp.set_tx_rate(sample_rate, self.tx_channel)
        self.usrp.set_tx_freq(uhd.libpyuhd.types.tune_request(tx_freq), self.tx_channel)
        self.usrp.set_tx_gain(tx_gain, self.tx_channel)
        # Set up the stream and receive buffer
        st_args = uhd.usrp.StreamArgs("fc32", "sc16")
        st_args.channels = [self.tx_channel]
        self.tx_streamer = self.usrp.get_tx_stream(st_args)
        self.rx_proc = None

        self.tx_queue = None
        self.rx_queues = None

        #print("Main class", self.rx_streamer)

    def rx_start(self, sf_list, complex_data_number = 3000000, block_size = 20):
        # THE DEFAULT MINIMUM BLOCK OF SAMPLES ROUGHLY CORRESPONDS TO 5 TIMES THE SIZE OF A 251 BYTES SF7 PACKET
        rx_packet_queue = np.empty((len(sf_list)), dtype=mp_queues.Queue)
        #print("SF LIST",sf_list)
        print("RX Start")
        for index in range(len(sf_list)):
            rx_packet_queue[index] = mp.Queue(0)
        self.rx_proc = threading.Thread(name = "Receiver", target=rx, args=(self.sample_rate, sf_list, self.bandwidth, self.receiving, rx_packet_queue, complex_data_number, self.rx_streamer, self.rx_pause_flag, block_size))
        self.rx_proc.start()
        self.rx_queues = rx_packet_queue
        return rx_packet_queue


    def rx_stop(self, wait = False):
        self.receiving.value = False
        if(wait):
            self.rx_proc.join()

    def rx_pause(self):
        self.rx_pause_flag.clear()

    def rx_resume(self):
        self.rx_pause_flag.set()


    def tx_send_burst(self, pkt_list, sleep_time, verbose = False):
        if(not self.sending.value):
            tx_burst_proc = threading.Thread(target=tx_burst, args=(self.sample_rate, self.tx_freq, pkt_list, sleep_time, self.sending, self.tx_streamer, self.signal_amplitude, verbose))
            tx_burst_proc.start()
            return tx_burst_proc
        else:
            print("The radio is already transmitting!")
            return None


    def tx_send_burst_multi_sf(self, pkt_list, sleep_time, sf_list):
        if(not self.sending.value):
            tx_burst_proc = threading.Thread(target=tx_burst_multi_sf, args=(self.sample_rate, self.tx_freq, pkt_list, sleep_time, self.sending, self.tx_streamer, self.signal_amplitude, sf_list))
            tx_burst_proc.start()
            return tx_burst_proc
        else:
            print("The radio is already transmitting!")
            return None



    def tx_start(self, sleep_time, verbose = False):

        if(not self.sending.value):
            self.tx_queue = mp.Queue(0)
            tx_proc = threading.Thread(name="Transmitter", target=tx, args=(self.sample_rate, self.tx_freq, self.tx_queue, sleep_time, self.sending, self.tx_streamer, self.signal_amplitude, verbose))
            tx_proc.start()
            return self.tx_queue


        else:
            print("TX is already ON!")
            return self.tx_queue

    def tx_stop(self):
        self.sending.value = False




        # print("Main class", self.rx_streamer)


############### END LORA_TRANSCEIVER.PY #########################

############### START LORA_UTILS.PY #########################

NACK_CODE = 255
POLLING_CODE = 254
POLLING_CODE_BROADCAST = 252
ACK_CODE = 253
BROADCAST_ID = 255
MAX_PACKET_SIZE = 251



def rate_calculator(sf, bw, cr):
    return sf * (4 / (4 + cr)) * (bw / (1 * np.power(2, sf)))


def gen_pack_polling(SF, BW, srcID, dstID, CR = 1, brdcst = False):
    payload = np.zeros((1,), dtype=np.uint8)
    if(brdcst):
        payload[0] = POLLING_CODE_BROADCAST
    else:
        payload[0] = POLLING_CODE
    return LoRaPacket(payload, srcID, dstID, seqn= 0, hdr_ok=1, has_crc=1, crc_ok=1,
                    cr=CR, ih=0, SF=SF, BW=BW, rssi=0, snr=0)

def pack_lora_data(data, SF, BW, packet_size, srcID, dstID, extended_sqn=True, CR = 1):
    if (extended_sqn):
        act_pkt_size = packet_size - 1
        pkt_group = -1

    else:
        act_pkt_size = packet_size
    data_bytes = data.view(dtype=np.uint8)
    n_packets = int(np.ceil(data_bytes.size / act_pkt_size))
    pack_array = np.empty(shape=(n_packets,), dtype=LoRaPacket)

    start = 0
    for index in range(n_packets):
        if (extended_sqn):
            chunk = data_bytes[start:start + act_pkt_size]
            payload = np.zeros((chunk.size + 1,), dtype=np.uint8)
            if (index % 256 == 0):
                pkt_group = pkt_group + 1
            payload[0] = pkt_group
            payload[1:chunk.size + 1] = chunk
        else:
            payload = data_bytes[start:start + act_pkt_size]

        pack_array[index] = LoRaPacket(payload, srcID, dstID, seqn=(index) % 256, hdr_ok=1, has_crc=1, crc_ok=1,
                                            cr=CR, ih=0, SF=SF, BW=BW, rssi=0, snr=0)
        start = start + act_pkt_size

    return pack_array


def pack_lora_nack(data, SF, BW, packet_size, srcID, dstID, CR = 1):

    act_pkt_size = packet_size - 1

    if(data.size == 0):
        n_packets = 1
    else:
        n_packets = int(np.ceil(data.size / act_pkt_size))
    pack_array = np.empty(shape=(n_packets,), dtype=LoRaPacket)
    data_bytes = data.view(dtype=np.uint8)
    start = 0
    for index in range(n_packets):
        chunk = data_bytes[start:start + act_pkt_size]
        payload = np.zeros((chunk.size + 1,), dtype=np.uint8)

        if (index == n_packets - 1):
            payload[0] = ACK_CODE
        else:
            payload[0] = NACK_CODE

        payload[1:chunk.size + 1] = chunk




        pack_array[index] = LoRaPacket(payload, srcID, dstID, seqn=(index) % 256, hdr_ok=1, has_crc=1, crc_ok=1,
                                            cr=CR, ih=0, SF=SF, BW=BW, rssi=0, snr=0)
        start = start + act_pkt_size



    return pack_array


def pack16bit(high_byte,low_byte):
    high_byte = np.uint8(high_byte)
    low_byte = np.uint8(low_byte)
    temp_arr = np.array(([low_byte, high_byte])).view(dtype = np.uint16)
    return temp_arr[0]



def unpack_lora_data(pkt_array, arr_type = np.uint8, extended_sqn = True):
    array_size = 0
    array_index = 0
    for pkt in pkt_array:
        array_size = array_size + pkt.payload.size
    if extended_sqn:
        array_size = array_size - pkt_array.size


    #print("Arr size",array_size)
    data_array = np.zeros((array_size,), dtype = np.uint8)

    for pkt in pkt_array:
        if extended_sqn:
            data_array[array_index : array_index + pkt.payload.size - 1] = pkt.payload[1:]
            array_index = array_index + pkt.payload.size - 1
        else:
            data_array[array_index:array_index + pkt.payload.size] = pkt.payload
            array_index = array_index + pkt.payload.size

        if(not(arr_type == np.uint8)):
            data_array = data_array.view(dtype = arr_type)

    return data_array

def unpack_lora_ack(acks_array):
    missing_seqn = np.zeros((250 * len(acks_array),), dtype= np.uint8)
    index = 0
    sqn_set = set()
    for pack in (acks_array):
        if pack.seqn in sqn_set:
            continue
        sqn_set.add(pack.seqn)
        pld_size = pack.payload.size
        if (pack.payload[0] == 255 and pld_size == 1):
            break
        missing_seqn[index: index + pld_size - 1] = pack.payload[1:]
        if(pld_size < MAX_PACKET_SIZE):
            missing_seqn =  missing_seqn[:index+pld_size - 1]
        index = index + pld_size - 1
    print(missing_seqn)
    #missing_seqn = missing_seqn[:index]
    try:
        missing_seqn = missing_seqn.view(dtype = np.uint16)
    except ValueError:
        missing_seqn = missing_seqn[:-1].view(dtype=np.uint16)



    return missing_seqn

############### END LORA_UTILS.PY #########################


############### START LORA.PY #########################

#############CONSTANTS#####################
# rising and falling edges duration
Trise = 50e-6  
# Carrier Frequency Offset as declared in the datasheet
Cfo_PPM = 17
# Receiver Noise Figure
NF_dB = 6
#Minimum duration for a LoRa packet
min_time_lora_packet = 20e-3 #20 milliseconds


##########################################################





def lora_packet(BW, OSF, SF, k1, k2, n_pr, IH, CR, MAC_CRC, SRC, DST, SEQNO, MESSAGE, Trise, t0_frac, phi0):
    # in our implementation, the payload includes a 4 byte pseudo-header: (SRC, DST, SEQNO, LENGTH)
    LENGTH = np.uint16(4 + (MESSAGE.size))

    # n_sym_hdr: number of chirps encoding the header (0 if IH: True)
    # n_bits_hdr: [bit DI PAYLOAD presenti nell'header CHIEDERE A STEFANO]
    (n_sym_hdr, n_bits_hdr) = lora_header_init(SF, IH)
    [PAYLOAD, n_sym_payload] = lora_payload_init(SF, LENGTH, MAC_CRC, CR, n_bits_hdr, DST, SRC, SEQNO, MESSAGE)

    # -------------------------------------------------BIT TO SYMBOL MAPPING
    payload_ofs = 0
    if IH:
        k_hdr = []
    else:
        [k_hdr, payload_ofs] = lora_header(SF, LENGTH, CR, MAC_CRC, PAYLOAD, payload_ofs)

    k_payload = lora_payload(SF, CR, n_sym_payload, PAYLOAD, payload_ofs)

    # --------------------------------- CSS MODULATION

    # number of samples per chirp
    K = np.power(2, SF)
    N = int(K * OSF)
    # number of samples in the rising/falling edge
    Nrise = int(np.ceil(Trise * BW * OSF))

    # preamble modulation
    (p, phi) = lora_preamble(n_pr, k1, k2, BW, K, OSF, t0_frac, phi0)

    # samples initialization
    s = np.concatenate((np.zeros(Nrise), p, np.zeros(N * (n_sym_hdr + n_sym_payload) + Nrise)))

    # rising edge samples
    s[0:Nrise] = p[N - 1 + np.arange(-Nrise + 1, 1, dtype=np.int16)] * np.power(
        np.sin(np.pi / 2 * np.arange(1, Nrise + 1) / Nrise), 2)
    s_ofs = p.size + Nrise

    # header modulation, if any
    for sym in range(0, n_sym_hdr):
        k = k_hdr[sym]
        (s[s_ofs + np.arange(0, N)], phi) = lora_chirp(+1, k, BW, K, OSF, t0_frac, phi)
        s_ofs = s_ofs + N

    # payload modulation
    for sym in range(0, n_sym_payload):
        k = k_payload[sym]
        (s[s_ofs + np.arange(0, N)], phi) = lora_chirp(+1, k, BW, K, OSF, t0_frac, phi)
        s_ofs = s_ofs + N

    # falling edge samples
    s[s_ofs + np.arange(0, Nrise)] = s[s_ofs + np.arange(0, Nrise)] * np.power(
        np.cos(np.pi / 2 * np.arange(0, Nrise) / Nrise), 2)

    return s, k_hdr, k_payload



def lora_header_init(SF, IH):
    if (IH):
        n_sym_hdr = np.uint16(0)
        n_bits_hdr = np.uint16(0)
    else:
        # interleaving block size, respectively for header & payload
        CR_hdr = np.uint16(4)
        DE_hdr = np.uint16(1)
        n_sym_hdr = 4 + CR_hdr
        intlv_hdr_size = (SF - 2 * DE_hdr) * (4 + CR_hdr)
        n_bits_hdr = np.uint16(intlv_hdr_size * 4 / (4 + CR_hdr) - 20)

    return n_sym_hdr, n_bits_hdr


# function [k_hdr,payload_ofs] =
def lora_header(SF, LENGTH, CR, MAC_CRC, PAYLOAD, payload_ofs):
    # header parity check matrix (reverse engineered through brute force search)

    header_FCS = np.array(([1, 1, 0, 0, 0], [1, 0, 1, 0, 0], [1, 0, 0, 1, 0], [1, 0, 0, 0, 1], [0, 1, 1, 0, 0],
                           [0, 1, 0, 1, 0], [0, 1, 0, 0, 1], [0, 0, 1, 1, 0], [0, 0, 1, 0, 1], [0, 0, 0, 1, 1],
                           [0, 0, 1, 1, 1],
                           [0, 1, 0, 1, 1]))

    CR_hdr = np.uint8(4)
    Hamming_hdr = np.array(([1, 0, 1, 1], [1, 1, 1, 0], [1, 1, 0, 1], [0, 1, 1, 1]), dtype=np.uint8)
    n_sym_hdr = 4 + CR_hdr
    DE_hdr = np.uint8(1)
    PPM = SF - 2 * DE_hdr
    gray_hdr = gray_lut(PPM)[0]
    intlv_hdr_size = PPM * n_sym_hdr

    # header (20 bit)
    LENGTH_bits = num2binary(LENGTH, 8)
    CR_bits = num2binary(CR, 3)
    hdr = np.concatenate((LENGTH_bits[np.arange(3, -1, -1, dtype=np.uint8)],
                          LENGTH_bits[np.arange(7, 3, -1, dtype=np.uint8)], np.array([MAC_CRC], dtype=np.uint8),
                          CR_bits[np.arange(2, -1, -1, dtype=np.uint8)], np.zeros(8, dtype=np.uint8)))
    hdr_chk_indexes = np.concatenate((np.arange(3, -1, -1, dtype=np.uint8), np.arange(7, 3, -1, dtype=np.uint8),
                                      np.arange(11, 7, -1, dtype=np.uint8)))

    hdr_chk = np.mod(hdr[hdr_chk_indexes] @ header_FCS, 2)
    hdr[12] = hdr_chk[0]
    hdr[16:20] = hdr_chk[4:-0:-1]

    # parity bit calculation
    C = np.zeros((PPM, 4 + CR_hdr))
    for k in range(0, 5):
        C[k, 0:4] = hdr[k * 4 + np.arange(0, 4)]
        C[k, 3 + np.arange(1, CR_hdr + 1)] = np.mod(C[k, 0:4] @ Hamming_hdr, 2)

    for k in range(5, PPM):
        C[k, 0:4] = PAYLOAD[payload_ofs + np.arange(0, 4, dtype=np.uint8)]
        payload_ofs = payload_ofs + 4
        C[k, 3 + np.arange(1, CR_hdr + 1)] = np.mod(C[k, 0:4] @ Hamming_hdr, 2)

    # rows flip
    C = np.flip(C, 0)

    S = np.zeros((4 + CR_hdr, PPM), dtype=np.uint8)
    for ii in range(0, PPM):
        for jj in range(0, 4 + CR_hdr):
            S[jj, np.mod(ii + jj, PPM)] = C[ii, jj]

    bits_hdr = np.reshape(S.transpose(), intlv_hdr_size, order='F')

    # bit to symbol mapping
    k_hdr = np.zeros(n_sym_hdr)
    K = np.power(2, SF)
    for sym in range(0, n_sym_hdr):
        k_hdr[sym] = K - 1 - np.power(2, (2 * DE_hdr)) * gray_hdr[
            bits_hdr[sym * PPM + np.arange(0, PPM, dtype=np.uint16)] @ np.power(2, np.arange(PPM - 1, -1, -1))]

    return k_hdr, payload_ofs



def lora_payload_init(SF, LENGTH, MAC_CRC, CR, n_bits_hdr, DST, SRC, SEQNO, MESSAGE):
    # bigger spreading factors (11 and 12) use 2 less bits per symbol
    if SF > 10:
        DE = np.uint8(1)
    else:
        DE = np.uint8(0)
    PPM = SF - 2 * DE
    n_bits_blk = PPM * 4
    n_bits_tot = 8 * LENGTH + 16 * MAC_CRC
    n_blk_tot = int(np.ceil((n_bits_tot - n_bits_hdr) / n_bits_blk))
    n_sym_blk = 4 + CR
    n_sym_payload = n_blk_tot * n_sym_blk

    byte_range = np.arange(7, -1, -1, dtype=np.uint8)
    PAYLOAD = np.zeros(int(n_bits_hdr + n_blk_tot * n_bits_blk), dtype=np.uint8)
    PAYLOAD[byte_range] = num2binary(DST, 8)
    PAYLOAD[8 + byte_range] = num2binary(SRC, 8)
    PAYLOAD[8 * 2 + byte_range] = num2binary(SEQNO, 8)
    PAYLOAD[8 * 3 + byte_range] = num2binary(LENGTH, 8)
    for k in range(0, MESSAGE.size):
        PAYLOAD[8 * (4 + k) + byte_range] = num2binary(MESSAGE[k], 8)

    if MAC_CRC:
        PAYLOAD[8 * LENGTH + np.arange(0, 16, dtype=np.uint8)] = CRC16(PAYLOAD[0:8 * LENGTH])

    # ----------------------------------------------------------- WHITENING
    W = np.array([1, 1, 1, 1, 1, 1, 1, 1], dtype=np.uint8)
    W_fb = np.array([0, 0, 0, 1, 1, 1, 0, 1], dtype=np.uint8)
    for k in range(1, int(np.floor(len(PAYLOAD) / 8) + 1)):
        PAYLOAD[(k - 1) * 8 + np.arange(0, 8, dtype=np.int32)] = np.mod(
            PAYLOAD[(k - 1) * 8 + np.arange(0, 8, dtype=np.int32)] + W, 2)
        W1 = np.array([np.mod(np.sum(W * W_fb), 2)])
        W = np.concatenate((W1, W[0:-1]))

    return PAYLOAD, n_sym_payload


#CRC-16 calculation for LoRa (reverse engineered from a Libelium board)
def CRC16(bits):
    length = int(len(bits) / 8)
    # initial states for crc16 calculation, valid for lenghts in the range 5,255
    state_vec = np.array([46885, 27367, 35014, 54790, 18706, 15954, \
                          9784, 59350, 12042, 22321, 46211, 20984, 56450, 7998, 62433, 35799, \
                          2946, 47628, 30930, 52144, 59061, 10600, 56648, 10316, 34962, 55618, \
                          57666, 2088, 61160, 25930, 63354, 24012, 29658, 17909, 41022, 17072, \
                          42448, 5722, 10472, 56651, 40183, 19835, 21851, 13020, 35306, 42553, \
                          12394, 57960, 8434, 25101, 63814, 29049, 27264, 213, 13764, 11996, \
                          46026, 6259, 8758, 22513, 43163, 38423, 62727, 60460, 29548, 18211, \
                          6559, 61900, 55362, 46606, 19928, 6028, 35232, 29422, 28379, 55218, \
                          38956, 12132, 49339, 47243, 39300, 53336, 29575, 53957, 5941, 63650, \
                          9502, 28329, 44510, 28068, 19538, 19577, 36943, 59968, 41464, 33923, \
                          54504, 49962, 64357, 12382, 44678, 11234, 58436, 47434, 63636, 51152, \
                          29296, 61176, 33231, 32706, 27862, 11005, 41129, 38527, 32824, 20579, \
                          37742, 22493, 37464, 56698, 29428, 27269, 7035, 27911, 55897, 50485, \
                          10543, 38817, 54183, 52989, 24549, 33562, 8963, 38328, 13330, 24139, \
                          5996, 8270, 49703, 60444, 8277, 43598, 1693, 60789, 32523, 36522, \
                          17339, 33912, 23978, 55777, 34725, 2990, 13722, 60616, 61229, 19060, \
                          58889, 43920, 9043, 10131, 26896, 8918, 64347, 42307, 42863, 7853, \
                          4844, 60762, 21736, 62423, 53096, 19242, 55756, 26615, 53246, 11257, \
                          2844, 47011, 10022, 13541, 18296, 44005, 23544, 18733, 23770, 33147, \
                          5237, 45754, 4432, 22560, 40752, 50620, 32260, 2407, 26470, 2423, \
                          33831, 34260, 1057, 552, 56487, 62909, 4753, 7924, 40021, 7849, \
                          4895, 10401, 32039, 40207, 63952, 10156, 53647, 51938, 16861, 46769, \
                          7703, 9288, 33345, 16184, 56808, 30265, 10696, 4218, 7708, 32139, \
                          34174, 32428, 20665, 3869, 43003, 6609, 60431, 22531, 11704, 63584, \
                          13620, 14292, 37000, 8503, 38414, 38738, 10517, 48783, 30506, 63444, \
                          50520, 34666, 341, 34793, 2623], dtype=np.uint16)
    crc_tmp = num2binary(state_vec[length - 5], 16)
    # crc_poly = [1 0 0 0 1 0 0 0 0 0 0 1 0 0 0 0 1]
    # 	for j = 1:numel(bits)/8
    # 		for k = 1:8
    # 			add = crc_tmp(1)
    # 			crc_tmp = [crc_tmp(2: ),bits((j-1)*8+9-k)]
    # 			if add
    # 				crc_tmp = mod(crc_tmp+crc_poly(2: ),2)
    #
    #
    #
    # 	CRC=crc_tmp(16:-1:1)
    pos = 0
    pos4 = 4
    pos11 = 11
    for j in range(0, length):
        for k in range(0, 8):
            add = crc_tmp[pos]
            crc_tmp[pos] = bits[j * 8 + 7 - k]
            if add:
                crc_tmp[pos4] = 1 - crc_tmp[pos4]
                crc_tmp[pos11] = 1 - crc_tmp[pos11]
                crc_tmp[pos] = 1 - crc_tmp[pos]

            pos = np.mod(pos + 1, 16)
            pos4 = np.mod(pos4 + 1, 16)
            pos11 = np.mod(pos11 + 1, 16)

    CRC = crc_tmp[np.mod(pos + np.arange(15, -1, -1, dtype=np.int32), 16)]
    return CRC


# gray and reversed-gray mappings

def gray_lut(n):
    pow_n = np.power(2, n)
    g = np.zeros(pow_n, dtype=np.uint16)

    vec = np.atleast_2d(np.arange(0, pow_n, dtype=np.uint16)).transpose()
    vec = np.flip((vec.view(np.uint8)), 1)
    vec = np.unpackbits(vec).reshape(pow_n, 16)
    vec = vec[:, -n:]
    vec = vec.transpose()
    support_x = np.zeros((n, pow_n))
    support_x[1:, :] = vec[:-1, :]

    ig = np.matmul(np.power(2, np.arange(n - 1, -1, -1)), np.mod(vec + support_x, 2))
    ig = ig.astype(int)
    g[ig] = np.arange(0, pow_n, dtype=np.uint16)
    return g, ig


# function k_payload = \
def lora_payload(SF, CR, n_sym_payload, PAYLOAD, payload_ofs):
    # varargout = {DST, SRC, SEQNO, MESSAGE}

    # hamming parity check matrices 
    Hamming_P1 = np.array(([1], [1], [1], [1]), dtype=np.uint8)
    Hamming_P2 = np.array(([1, 0], [1, 1], [1, 1], [0, 1]), dtype=np.uint8)
    Hamming_P3 = np.array(([1, 0, 1], [1, 1, 1], [1, 1, 0], [0, 1, 1]), dtype=np.uint8)
    Hamming_P4 = np.array(([1, 0, 1, 1], [1, 1, 1, 0], [1, 1, 0, 1], [0, 1, 1, 1]), dtype=np.uint8)

    if CR == 1:
        Hamming = Hamming_P1
    elif CR == 2:
        Hamming = Hamming_P2
    elif CR == 3:
        Hamming = Hamming_P3
    elif CR == 4:
        Hamming = Hamming_P4

    if SF > 10:
        DE = 1
    else:
        DE = 0

    PPM = SF - 2 * DE
    n_sym_blk = (4 + CR)
    intlv_blk_size = PPM * n_sym_blk
    gray = gray_lut(PPM)[0]
    K = np.power(2, SF)
    n_blk_tot = int(n_sym_payload / n_sym_blk)
    C = np.zeros((PPM, 4 + CR))
    S = np.zeros((4 + CR, PPM))
    k_payload = np.zeros(n_sym_payload)
    for blk in range(0, n_blk_tot):
        for k in range(0, PPM):
            C[k, 0:4] = PAYLOAD[payload_ofs + np.arange(0, 4, dtype=np.uint8)]
            payload_ofs = payload_ofs + 4
            C[k, 4: 4 + CR] = np.mod(C[k, 0:4] @ Hamming, 2)

        # row flip
        C = np.flip(C, 0)

        # interleaving
        for ii in range(0, PPM):
            for jj in range(0, 4 + CR):
                S[jj, np.mod(ii + jj, PPM)] = C[ii, jj]

        bits_blk = np.reshape(S.transpose(), intlv_blk_size, order='F')

        # bit to symbol mapping
        for sym in range(0, n_sym_blk):
            k_payload[(blk) * n_sym_blk + sym] = K - 1 - np.power(2, (2 * DE)) * gray[int(
                bits_blk[(sym * PPM + np.arange(0, PPM, dtype=np.uint16))] @ np.power(2, np.arange(PPM - 1, -1, -1)))]

    return k_payload





def chirp(f_ini,N,Ts,Df,t0_frac = 0,phi_ini = 0):
    t = (t0_frac+np.arange(0,N, dtype = np.int32))*Ts
    T = N*Ts
    s = np.exp(1j*(phi_ini + 2*np.pi*f_ini*t + np.pi*Df*np.power(t,2)))
    phi_fin = phi_ini + 2*np.pi*f_ini*T + np.pi*Df*np.power(T,2)
    return s, phi_fin


def lora_packet_rx(s,SF,BW,OSF,Trise,p_ofs_est,Cfo_est):
    truncated = False
    fs = BW*OSF
    Ts = 1/fs
    N = np.power(2,SF)*OSF
    Nrise = np.ceil(Trise*fs)
    d0 = chirp(BW/2,N,Ts,-BW/(N*Ts))[0]
    DST = -1
    SRC = -1
    SEQNO = -1
    CR = -1
    HAS_CRC = -1
    # demodula i simboli dell'header
    # n_sym_hdr == 8 o 0 in caso di header implicito
    # numero di campioni del preambolo
    ofs = int(Nrise+12.25*N)

    CR_hdr = 4
    n_sym_hdr = 4+CR_hdr
    k_hdr_est = np.zeros((n_sym_hdr))
    for i in range(0,n_sym_hdr):

        try:

            temp = np.exp(-1j * 2 * np.pi * Cfo_est * Ts * (ofs+np.arange(0,N))) * s[p_ofs_est+ofs+np.arange(0,N, dtype = np.int32)] *d0
        except IndexError:
            k_hdr_est = None
            MAC_CRC_OK = False
            HDR_FCS_OK = False
            k_payload_est = None
            MSG = None
            truncated = True
            return k_hdr_est,HDR_FCS_OK,k_payload_est,MAC_CRC_OK,MSG,DST,SRC,SEQNO,CR,HAS_CRC,truncated,ofs


        ofs = ofs+N
        pos = np.argmax(np.abs(np.fft.ifft(temp[0:-1:OSF])))
        k_hdr_est[i]=pos-1


    #in case of header checksum failure, we assume the message to be lost/corrupted [ToDo: implement implicit header mode]
    (HDR_FCS_OK,LENGTH,HAS_CRC,CR,PAYLOAD_bits_hdr) = lora_header_decode(SF,k_hdr_est)
    if not HDR_FCS_OK:
        k_hdr_est = None
        MAC_CRC_OK = False
        k_payload_est = None
        MSG = None

        return k_hdr_est,HDR_FCS_OK,k_payload_est,MAC_CRC_OK,MSG,DST,SRC,SEQNO,CR,HAS_CRC,truncated,ofs
    else:
        n_bits_hdr = (PAYLOAD_bits_hdr.size)
        n_sym_payload = lora_payload_n_sym(SF,LENGTH,HAS_CRC,CR,n_bits_hdr)

        k_payload_est = np.zeros((n_sym_payload))
        for i in range(0, n_sym_payload):
            try:
                temp = np.exp(-1j*2*np.pi*Cfo_est*Ts*(ofs+np.arange(0,N, dtype = np.int32))) * s[p_ofs_est+ofs+np.arange(0,N, dtype = np.int32)] * d0
            except:
                k_hdr_est = None
                MAC_CRC_OK = False
                k_payload_est = None
                MSG = None

                truncated = True
                return k_hdr_est,HDR_FCS_OK,k_payload_est,MAC_CRC_OK,MSG,DST,SRC,SEQNO,CR,HAS_CRC,truncated,ofs

            ofs = ofs+N
            pos = np.argmax(np.abs(np.fft.ifft(temp[1:-1:OSF])))
            k_payload_est[i] = pos-1


        (MAC_CRC_OK,DST,SRC,SEQNO,MSG,HAS_CRC) = lora_payload_decode(SF,k_payload_est,PAYLOAD_bits_hdr,HAS_CRC,CR,LENGTH)
        # if MAC_CRC_OK:
        #
        # 	#fprintf('DST: #d SRC: #d SEQ: #d LEN: #d DATA: "',...
        # 		#DST,SRC,SEQNO,LENGTH)
        # 	for i in range(1,len(MSG)+1):
        # 		if MSG[i-1]>=32 and MSG[i-1]<=127:
        # 			#fprintf('#c',MSG(i))
        # 			pass
        # 		else:
        # 			pass
        # 			#fprintf('\\#d',MSG(i))
        return k_hdr_est, HDR_FCS_OK, k_payload_est, MAC_CRC_OK, MSG, DST,SRC,SEQNO, CR,HAS_CRC, truncated,ofs






def lora_header_decode(SF,k_hdr):

    if SF<7:
        FCS_OK = False
        return






    DE_hdr = 1
    PPM = SF-2*DE_hdr
    (_,degray) = gray_lut(PPM)

    #np.fft(downsample(s_rx.*chirp)) demodulates the signal
    CR_hdr = 4
    n_sym_hdr = 4+CR_hdr
    intlv_hdr_size = PPM*n_sym_hdr
    bits_est = np.zeros((intlv_hdr_size), dtype = np.uint8)
    K = np.power(2,SF)
    for sym in range(0,n_sym_hdr):
        # gray decode
        bin = int(np.round((K-1-k_hdr[sym])/4))
        bits_est[(sym)*PPM+np.arange(0,PPM, dtype = np.int32)] = num2binary(degray[np.mod(bin,np.power(2,PPM))],PPM)


    # interleaver del brevetto
    S = np.reshape(bits_est,(PPM,4+CR_hdr),order = 'F').transpose()
    C = np.zeros((PPM,4+CR_hdr),dtype = np.uint8)
    for ii in range(0,PPM):
        for jj in range (0,4+CR_hdr):
            C[ii,jj] = S[jj,np.mod(ii+jj,PPM)]



    # row flip
    C = np.flip(C,0)



    # header parity check matrix (reverse engineered through brute force search)
    header_FCS = np.array(([1,1,0,0,0],[1,0,1,0,0],[1,0,0,1,0],[1,0,0,0,1],[0,1,1,0,0], \
        [0,1,0,1,0],[0,1,0,0,1],[0,0,1,1,0],[0,0,1,0,1],[0,0,0,1,1],[0,0,1,1,1], \
        [0,1,0,1,1]))

    FCS_chk = np.mod(np.concatenate((C[0,np.arange(3,-1,-1, dtype = np.int32)],C[1,np.arange(3,-1,-1)],C[2,np.arange(3,-1,-1, dtype = np.int32)],np.array([C[3,0]],dtype=np.uint8),C[4,np.arange(3,-1,-1, dtype = np.int32)])) @ [np.concatenate((header_FCS,np.eye(5)),0)],2)
    FCS_OK = not np.any(FCS_chk)
    if not FCS_OK:
        LENGTH = -1
        HAS_CRC = False
        CR = -1
        PAYLOAD_bits = -1
      

    else:
        # header decoding
        # Header=char(['len:',C(1,4:-1:1)+'0',C(2,4:-1:1)+'0',...
        # 	' CR:',C(3,4:-1:2)+'0',...
        # 	' MAC-CRC:',C(3,1)+'0',...
        # 	' HDR-FCS:',C(4,4:-1:1)+'0',C(5,4:-1:1)+'0'])
        #fprintf('Header Decode: #s [OK]\n',Header)


        LENGTH = bit2uint8(np.concatenate((C[1,0:4],C[0,0:4])))
        HAS_CRC = C[2,0] # HAS_CRC
        CR = bit2uint8(C[2,1:4])
        FCS_HDR = bit2uint8(np.concatenate((C[4, 0:4], C[3, 0:4])))

        n_bits_hdr = PPM*4-20
        PAYLOAD_bits=np.zeros((1,n_bits_hdr),dtype=np.uint8)
        for i in range(5,PPM):
            #C(i,4+(1:CR_hdr)) = mod(C(i,1:4)*Hamming_hdr,2)
            PAYLOAD_bits[0,(i-5)*4+np.arange(0,4, dtype = np.int32)]=C[i,0:4]


    return FCS_OK,LENGTH,HAS_CRC,CR,PAYLOAD_bits


def num2binary(num,length = 0):

    num = np.array([num],dtype=np.uint16)
    num = np.flip(num.view(np.uint8))
    num = np.unpackbits(num)
    return num[-length:]



def lora_payload_n_sym(SF,LENGTH,MAC_CRC,CR,n_bits_hdr):

    # bigger spreading factors (11 and 12) use 2 less bits per symbol
    if SF > 10:
        DE = 1

    else:
        DE = 0
    PPM = SF-2*DE
    n_bits_blk = PPM*4
    n_bits_tot = 8*LENGTH+16*MAC_CRC
    n_blk_tot = np.ceil((n_bits_tot-n_bits_hdr)/n_bits_blk)
    n_sym_blk = 4+CR
    n_sym_payload = int(n_blk_tot*n_sym_blk)
    return n_sym_payload




def lora_payload_decode(SF,k_payload,PAYLOAD_hdr_bits,HAS_CRC,CR,LENGTH_FROM_HDR):


    # hamming parity check matrices 
    Hamming_P1 = np.array(([1],[1],[1],[1]), dtype=np.uint8)
    Hamming_P2 = np.array(([1,0], [1,1], [1,1], [0,1]), dtype=np.uint8)
    Hamming_P3 = np.array(([1,0,1], [1,1,1], [1,1,0], [0,1,1]), dtype=np.uint8)
    Hamming_P4 = np.array(([1,0,1,1], [1,1,1,0], [1,1,0,1], [0,1,1,1]),dtype = np.uint8)

    if CR == 1:
        Hamming = Hamming_P1
    elif CR == 2:
        Hamming = Hamming_P2
    elif CR == 3:
        Hamming = Hamming_P3
    elif CR == 4:
        Hamming = Hamming_P4

    if SF > 10:
        DE = 1
    else:
        DE = 0
    PPM = SF-2*DE
    n_sym_blk = (4+CR)
    n_bits_blk = PPM*4
    intlv_blk_size = PPM*n_sym_blk
    [_,degray] = gray_lut(PPM)
    K = np.power(2,SF)
    n_sym_payload = len(k_payload)
    n_blk_tot = int(n_sym_payload/n_sym_blk)
    try:
        PAYLOAD = np.concatenate((np.squeeze(PAYLOAD_hdr_bits),np.zeros((int(n_bits_blk*n_blk_tot)), dtype = np.uint8)))
    except ValueError:
        PAYLOAD = np.zeros((int(n_bits_blk*n_blk_tot)), dtype = np.uint8)
    payload_ofs = (PAYLOAD_hdr_bits.size)
    for blk in range (0,n_blk_tot):

        bits_blk = np.zeros((intlv_blk_size))
        for sym in range(0,n_sym_blk):
            bin = round((K-2-k_payload[(blk)*n_sym_blk+sym])/np.power(2,(2*DE)))
            # gray decode
            bits_blk[(sym)*PPM+np.arange(0,PPM, dtype = np.int32)] = num2binary(degray[int(np.mod(bin,np.power(2,PPM)))],PPM)


        # interleaving



        S = np.reshape(bits_blk,(PPM, (4+CR)),order = 'F').transpose()
        C = np.zeros((PPM,4+CR),dtype=np.uint8)
        for ii in range(0,PPM):
            for jj in range(0,4+CR):
                C[ii,jj]= S[jj,np.mod(ii+jj,PPM)]


        # row flip
        C = np.flip(C,0)



        for k in range(0,PPM):
            PAYLOAD[payload_ofs+np.arange(0,4, dtype = np.int32)] = C[k,0:4]
            payload_ofs = payload_ofs+4



    # ----------------------------------------------------------- WHITENING
    W = np.array([1,1,1,1,1,1,1,1], dtype=np.uint8)
    W_fb = np.array([0,0,0,1,1,1,0,1], dtype = np.uint8)
    for k in range (1,int(np.floor(len(PAYLOAD)/8) + 1)):
        PAYLOAD[(k-1)*8+np.arange(0,8, dtype = np.int32)] = np.mod(PAYLOAD[(k-1)*8+np.arange(0,8, dtype = np.int32)]+W,2)
        W1 = np.array([np.mod(np.sum(W*W_fb),2)])
        W = np.concatenate((W1,W[0:-1]))




    #NOTE HOW THE TOTAL LENGTH IS 4 BYTES + THE PAYLOAD LENGTH
    #INDEED, THE FIRST 4 BYTES ENCODE DST, SRC, SEQNO AND LENGTH INFOS
    DST = bit2uint8(PAYLOAD[0:8])
    SRC = bit2uint8(PAYLOAD[8+np.arange(0,8, dtype = np.int32)])
    SEQNO = bit2uint8(PAYLOAD[8*2+np.arange(0,8, dtype = np.int32)])
    LENGTH = bit2uint8(PAYLOAD[8*3+np.arange(0,8, dtype = np.int32)])
    if (LENGTH == 0):
        LENGTH = LENGTH_FROM_HDR
    
    MSG_LENGTH = LENGTH-4
    if (((LENGTH+2)*8 > len(PAYLOAD) and HAS_CRC) or (LENGTH*8 > len(PAYLOAD) and not (HAS_CRC)) or LENGTH<4):
        MAC_CRC_OK = False

        return MAC_CRC_OK, DST, SRC, SEQNO, None, HAS_CRC

    MSG=np.zeros((int(MSG_LENGTH)), dtype = np.uint8)
    for i in range (0,int(MSG_LENGTH)):
        MSG[i]=bit2uint8(PAYLOAD[8*(4+i)+np.arange(0,8, dtype = np.int32)])

    if not HAS_CRC:
        MAC_CRC_OK = True
    else:
        #fprintf('CRC-16: 0x#02X#02X ',...
            #PAYLOAD(8*LENGTH+(1:8))*2.^(0:7)',...
            #PAYLOAD(8*LENGTH+8+(1:8))*2.^(0:7)')
        temp = CRC16(PAYLOAD[0:LENGTH*8])
        temp = np.power(2,np.arange(0,8, dtype = np.int32)) @ (np.reshape(temp,(8,2),order = 'F'))
        #fprintf('(CRC-16: 0x#02X#02X)',temp(1),temp(2))

        if np.any(PAYLOAD[8*LENGTH+np.arange(0,16, dtype = np.int32)] != CRC16(PAYLOAD[0:8*LENGTH])):
            #fprintf(' [CRC FAIL]\n')
            MAC_CRC_OK = False
        else:
            #fprintf(' [CRC OK]\n')
            MAC_CRC_OK = True



    return MAC_CRC_OK,DST,SRC,SEQNO,MSG,HAS_CRC




def bit2uint8(bits):
    return np.packbits(bits, bitorder='little')[0]




# LoRa preamble (reverse engineered from a real signal, is actually different from the one described in the patent)
def lora_preamble(n_preamble, k1, k2, BW, K, OSF, t0_frac=0, phi0=0):
    (u0, phi) = lora_chirp(+1, 0, BW, K, OSF, t0_frac, phi0)
    (u1, phi) = lora_chirp(+1, k1, BW, K, OSF, t0_frac, phi)
    (u2, phi) = lora_chirp(+1, k2, BW, K, OSF, t0_frac, phi)
    (d0, phi) = lora_chirp(-1, 0, BW, K, OSF, t0_frac, phi)
    (d0_4, phi) = chirp(BW / 2, K * OSF / 4, 1 / (BW * OSF), -np.power(BW, 2) / K, t0_frac, phi)
    s = np.concatenate((np.tile(u0, (n_preamble)), u1, u2, d0, d0, d0_4))
    return s, phi


#generate a lora chirp
def lora_chirp(mu, k, BW, K, OSF, t0_frac=0, phi0=0):
    fs = BW * OSF
    Ts = 1 / fs
    # number of samples in one period T
    N = K * OSF
    T = N * Ts
    # derivative of the instant frequency
    Df = mu * BW / T
    if k > 0:
        (s1, phi) = chirp(mu * BW * (1 / 2 - k / K), k * OSF, Ts, Df, t0_frac, phi0)
        (s2, phi) = chirp(-mu * BW / 2, (K - k) * OSF, Ts, Df, t0_frac, phi)
        s = np.concatenate((s1, s2))
    else:
        (s, phi) = chirp(-mu * BW / 2, K * OSF, Ts, Df, t0_frac, phi0)
    return s, phi

def calculate_power(samples):
    return np.abs(samples)**2

def calculate_mean_power(samples):
    return np.mean(calculate_power(samples))

def estimate_noise_power(samples):
    total_power = np.sum(calculate_power(samples))
    samples_power = calculate_mean_power(samples)
    return total_power - samples_power

def calculate_snr(samples):
    samples_power = calculate_mean_power(samples)
    noise_power = estimate_noise_power(samples)
    return 10 * np.log10(samples_power / noise_power)

def calculate_rssi(samples):
    samples_power = calculate_mean_power(samples)
    return 10 * np.log10(samples_power)

def samples_decoding(s,BW,N,Ts,K,OSF,Nrise,SF,Trise):

    max_packets = int(np.ceil(Ts * s.size / min_time_lora_packet))
    pack_array = np.empty(shape=(max_packets,), dtype=LoRaPacket)



    OSF = int(OSF)
    N = int(N)
    K = int(K)
    SF = int(SF)
    cumulative_index = 0
    last_index = 0
    received = 0




    while True:
        if s.size < N:
            print("size")
            break

        (success, payload, last_index, truncated, HDR_FCS_OK, MAC_CRC_OK, DST, SRC, SEQNO, CR, HAS_CRC, offset) = rf_decode(s[cumulative_index:] ,BW,N,Ts,K,OSF,Nrise,SF,Trise)

        if truncated:
            # print("Truncated")
            break

        if(success):
            # print("success")
            # print(payload)
            # print("message","".join([chr(int(item)) for item in payload]))
            # print("FCS Check", HDR_FCS_OK)
            # print("MAC CRC",MAC_CRC_OK)
            # print("PAYLOAD LENGTH", len(payload))
            rssi = calculate_rssi(s[cumulative_index:])
            snr = calculate_snr(s[cumulative_index:])
            pack_array[received] = LoRaPacket(payload,SRC,DST,SEQNO,HDR_FCS_OK,HAS_CRC,MAC_CRC_OK,CR,0,SF,BW, 0, 0)
            received = received + 1



        if(last_index == -1):
            break

        else:
            #cumulative_index = cumulative_index + last_index + 10*N
            #cumulative_index = cumulative_index + last_index + 28 * N
            if (offset != -1):
                cumulative_index = cumulative_index + last_index + offset
            else:
                cumulative_index = cumulative_index + last_index + 28 * N
    return pack_array[:received]





def rf_decode(s,BW,N,Ts,K,OSF,Nrise,SF,Trise):
    truncated = False
    payload = None
    last_index = -1
    success = False
    HDR_FCS_OK = None
    MAC_CRC_OK = None
    DST = -1
    SRC = -1
    SEQNO = -1
    CR = -1
    HAS_CRC = -1
    ns = len(s)
    # base upchirp & downchirp
    u0 = chirp(-BW/2,N,Ts,BW/(N*Ts))[0]

    d0 = np.conj(u0)

    # parameters and state variables in the synch block
    m_phases = 2
    m_phase = 0
    m_vec = -1*np.ones((2*m_phases,6)) # peak values positioning in the DFT
    # oversampling factor for fine-grained esteem
    OSF_fine_sync = 4

    missed_sync = True
    sync_metric = np.Inf

    offset = -1
    main_loop = True
    s_ofs = 0

    #         s = s(620000:1000000)
    #         ns = numel(s)
    while main_loop:
      
        # sample window for a full chirp
        try:

            s_win = s[np.arange(s_ofs,s_ofs+N, dtype = np.int32)]
        except IndexError:
            success = False
            payload = None

            return success, payload, last_index, truncated, HDR_FCS_OK, MAC_CRC_OK, DST, SRC, SEQNO, CR, HAS_CRC, offset

        # multiplication of the signal with a downchirp and a upchirp, respectively.





        Su = np.abs(np.fft.fft(s_win * d0))
        Sd = np.abs(np.fft.fft(s_win * u0))
        m_u=np.argmax(Su)
        m_d=np.argmax(Sd)

        # convert the positioning in values in the range [-N/2,N/2-1] 
        m_u=np.mod(m_u-1+N/2,N)-N/2
        m_d=np.mod(m_d-1+N/2,N)-N/2


        m_vec[np.arange(m_phase * 2, m_phase * 2 + 2), 1:] = m_vec[np.arange(m_phase * 2, m_phase * 2 + 2), :-1]
        m_vec[np.arange(m_phase * 2, m_phase * 2 + 2), 0] = np.array([m_u, m_d]) #Numpy automatically converts the row into a column





        # three upchirpsm followed by two upchirps shifted by
        # 8 e 16 bits, further followed by two downchirps, correspond to
        # a preamble
        if np.abs(m_vec[m_phase*2+1,0]-m_vec[m_phase*2+1,1])<=1 and \
                np.abs(m_vec[m_phase*2,2]-m_vec[m_phase*2,3]-8)<=1 and \
                np.abs(m_vec[m_phase*2,3]-m_vec[m_phase*2,4]-8)<=1 and \
                np.abs(m_vec[m_phase*2,4]-m_vec[m_phase*2,5]) <=1:

            missed_sync = False
            #keyboard
            tmp = np.sum(np.abs(m_vec[m_phase*2+1,1:2])+np.abs(m_vec[m_phase*2,5:6]))
            if tmp < sync_metric:
                sync_metric = tmp

                #fprintf('phase: #g\n',m_phase)
                #display(m_vec(m_phase*2+(1:2),:))
                Nu = 2
                # fine-grained estimation of the positions of the maximums in the DFT 
                m_u0 = 0
                for i in range (1,Nu+1):
                    try:
                        Su = np.abs(np.fft.fft(((s[np.arange(s_ofs - (4+i) * N,s_ofs - (4+i) * N + N, dtype = np.int32)]) * d0),N*OSF_fine_sync))
                    except IndexError:
                        truncated = True
                        return success, payload, last_index, truncated, HDR_FCS_OK, MAC_CRC_OK, DST, SRC, SEQNO, CR, HAS_CRC, offset

                    m_u = np.argmax(Su)
                    if (m_u > 0 and m_u < N*OSF_fine_sync-1):
                        m_u = m_u + 0.5*(Su[m_u-1]-Su[m_u+1]) / (Su[m_u-1]-2*Su[m_u]+Su[m_u+1])

                    m_u0 = m_u0 + np.mod(m_u-1+N*OSF_fine_sync/2,N*OSF_fine_sync)-N*OSF_fine_sync/2

                m_u0 = m_u0/Nu

                Nd = 2
                m_d0 = 0
                for i in range(1,Nd+1):
                    Sd = np.abs(np.fft.fft(((s[np.arange(s_ofs - (i - 1) * N, s_ofs - (i - 1) * N + N, dtype = np.int32)]) * u0), int(N * OSF_fine_sync)))
                    m_d = np.argmax(Sd)
                    if m_d > 1 and m_d < N*OSF_fine_sync:
                        try:
                            m_d = m_d + 0.5*(Sd[m_d-1]-Sd[m_d+1]) / (Sd[m_d-1]-2*Sd[m_d]+Sd[m_d+1])
                        except IndexError:
                            pass

                    m_d0 = m_d0 + np.mod(m_d-1+N*OSF_fine_sync/2,N*OSF_fine_sync)-N*OSF_fine_sync/2

                m_d0 = m_d0/Nd

                # Cfo_est: frequency error
                #t_est: timing error
                Cfo_est = (m_u0+m_d0)/2*BW/K/OSF_fine_sync
                t_est = (m_d0-m_u0)*OSF/2/OSF_fine_sync + s_ofs-11*N-Nrise # n_pr = 8 + 2 syncword + 1 downchirp
                break





        m_phase = np.mod(m_phase+1,m_phases)
        s_ofs = s_ofs + N/m_phases
        if s_ofs+N > ns:
            main_loop = False
            success = False
            return success, payload, last_index, truncated, HDR_FCS_OK, MAC_CRC_OK, DST, SRC, SEQNO, CR, HAS_CRC, offset



    if not missed_sync:

        missed_sync = True

       
       #symbol-level receiver
        p_ofs_est = int(np.ceil(t_est))

        last_index = p_ofs_est

        t0_frac_est = np.mod(-t_est,1)
        #keyboard
        (k_hdr_est,HDR_FCS_OK,k_payload_est,MAC_CRC_OK,MSG,DST,SRC,SEQNO,CR,HAS_CRC,truncated,offset) = lora_packet_rx(s,SF,BW,OSF,Trise,p_ofs_est,Cfo_est)


        if (not truncated) and (HDR_FCS_OK and MAC_CRC_OK):
            n_sym_hdr = len(k_hdr_est)
            n_sym_payload = len(k_payload_est)
            rx_success = True
            success = True
            payload = MSG

        return success, payload, last_index, truncated, HDR_FCS_OK, MAC_CRC_OK, DST, SRC, SEQNO, CR, HAS_CRC, offset


#SUPPORT FUNCTION TO ENCODE A LORA PACKET
def complex_lora_packet(K, n_pr, IH, CR, MAC_CRC, SRC, DST, SEQNO, BW, OSF, SF, Trise, N, Ts, fs, Cfo_PPM, f0,  MESSAGE, t0_frac, phi0):
    k1 = K-8
    k2 = K-16
    SF = np.uint8(SF)
    p = lora_packet(BW,OSF,SF,k1,k2,n_pr,IH,CR,MAC_CRC,SRC,DST,SEQNO,MESSAGE,Trise,t0_frac,phi0)[0]
    size_p = p.size
     
    ntail = N
    Ttail = ntail*Ts
    T = np.ceil(size_p*Ts+Ttail)   # WHOLE NUMBER OF SECONDS
    ns = int(T*fs)
    s = np.zeros(ns, dtype= np.complex64)
     #p_ofs=ceil((ns-np-ntail)*rand)
    p_ofs = 10000

    t = np.arange(0,size_p)*Ts
    # Cfo_TX = Cfo_PPM*1e-6*f0*(2*rand-1)
    # Cfo = Cfo_TX+Cfo_PPM*1e-6*f0*(2*rand-1)
    Cfo_TX = Cfo_PPM * 1e-6 * f0 * 1
    Cfo = Cfo_TX + Cfo_PPM * 1e-6 * f0 * 1
    s[p_ofs+np.arange(0,size_p)] = s[p_ofs+np.arange(0,size_p)] + p * np.exp(1j*(2*np.pi*Cfo*t+phi0))
    s = s[np.arange(0,p_ofs+size_p)]
    return s



#ENCODER FUNCTION. GIVEN A PAYLOAD (IN BYTES), AND THE INTENDED TRANSMISSION PARAMETERS, GENERATES THE SAMPLES FOR THE CORRESPONDING LORA PACKET
def encode(f0, SF, BW, payload, fs, src, dst, seqn, cr=1, enable_crc=1, implicit_header=0, preamble_bits=8):

    OSF = fs / BW
    Ts = 1 / fs
    Nrise = np.ceil(Trise * fs)
    K = np.power(2, SF)
    N = K * OSF
    # t0_frac = rand
    t0_frac = 0
    # phi0 = 2*pi*rand
    phi0 = 0
    complex_samples = complex_lora_packet(K, preamble_bits, implicit_header, cr, enable_crc, src, dst, seqn, BW, OSF, SF, Trise, N,
                            Ts, fs, Cfo_PPM, f0, payload, t0_frac, phi0)
    return complex_samples

#DECODER FUNCTION. LOOKS FOR LORA PACKETS IN THE INPUT COMPLEX SAMPLES. RETURNS ALL THE PACKETS FOUND IN THE SAMPLES.

def decode(complex_samples,SF, BW, fs):
    OSF = fs / BW
    Ts = 1 / fs
    Nrise = np.ceil(Trise * fs)

    K = np.power(2, SF)
    N = K * OSF
    return samples_decoding(complex_samples, BW, N, Ts, K, OSF, Nrise, SF, Trise)


#CLASS TO CONVENIENTLY ENCAPSULATE LORA PACKETS
class LoRaPacket:
    def __init__(self,payload,src,dst,seqn,hdr_ok,has_crc,crc_ok,cr,ih,SF,BW, rssi, snr):
        self.payload = payload
        self.src = np.uint8(src)
        self.dst = np.uint8(dst)
        self.seqn = np.uint8(seqn)
        self.hdr_ok = np.uint8(hdr_ok)
        self.has_crc = np.uint8(has_crc)
        self.crc_ok = np.uint8(crc_ok)
        self.cr = np.uint8(cr)
        self.ih = np.uint8(ih)
        self.SF = np.uint8(SF)
        self.BW = BW
        self.rssi = rssi
        self.snr = snr

    def __eq__(self, other):
        payload_eq = np.all(self.payload == other.payload)
        src_eq = (self.src == other.src)
        dst_eq = (self.dst == other.dst)
        seqn_eq = (self.seqn == other.seqn)
        hdr_ok_eq  = (self.hdr_ok == other.hdr_ok)
        has_crc_eq = (self.has_crc == other.has_crc)
        crc_ok_eq = (self.crc_ok == other.crc_ok)
        cr_eq = (self.cr == other.cr)
        ih_eq = (self.ih == other.ih)
        SF_eq = (self.SF == other.SF)
        BW_eq = (self.BW == other.BW)

        return payload_eq and src_eq and dst_eq and seqn_eq and hdr_ok_eq and has_crc_eq and crc_ok_eq and cr_eq and ih_eq and SF_eq and BW_eq

    def __repr__(self):
        desc = "LoRa Packet Info:\n"
        sf_desc = "Spreading Factor: " + str(self.SF) + "\n"
        bw_desc = "Bandwidth: " + str(self.BW) + "\n"
        if not self.hdr_ok:
            hdr_chk = "Header Integrity Check Failed" + "\n"
            return desc + sf_desc + bw_desc + hdr_chk

        else:
            if self.ih:
                ih_desc = "Implicit Header ON" + "\n"
                if self.has_crc:
                    if self.crc_ok:
                        crc_check = "Payload Integrity Check OK" + "\n"
                        pl_str = "Payload: " + str(self.payload) + "\n"
                        pl_len = "Payload Length: " + str(self.payload.size) + "\n"
                        return desc + sf_desc + bw_desc + ih_desc + crc_check + pl_len + pl_str
                    else:
                        crc_check = "Payload Integrity Check Failed" + "\n"
                        return desc + sf_desc + bw_desc + ih_desc + crc_check
                else:
                    crc_check = "CRC Disabled for this packet. Payload may be corrupted" + "\n"
                    pl_str = "Payload: " + str(self.payload) + "\n"
                    pl_len = "Payload Length: " + str(self.payload.size) + "\n"
                    return desc + sf_desc + bw_desc + ih_desc +  pl_len + pl_str
            else:
                ih_desc = "Explicit Header ON" + "\n"
                hdr_chk = "Header Integrity Check OK" + "\n"
                src_desc = "Source: " + str(self.src) + "\n"
                dest_desc = "Destination: " + str(self.dst) + "\n"
                seq_desc = "Sequence number: " + str(self.seqn) + "\n"
                cr_desc = "Coding Rate: " + str(self.cr) + "\n"
                if self.has_crc:
                    if self.crc_ok:
                        crc_check = "Payload Integrity Check OK" + "\n"
                        pl_str = "Payload: " + str(self.payload) + "\n"
                        pl_len = "Payload Length: " + str(self.payload.size) + "\n"
                        return desc + sf_desc + bw_desc + hdr_chk + ih_desc + src_desc + dest_desc + seq_desc + cr_desc + crc_check + pl_len + pl_str
                    else:
                        crc_check = "Payload Integrity Check Failed" + "\n"
                        return desc + sf_desc + bw_desc + hdr_chk + ih_desc + src_desc + dest_desc + seq_desc + cr_desc + crc_check
                else:
                    crc_check = "CRC Check Disabled for this packet. Payload may be corrupted." + "\n"
                    pl_str = "Payload: " + str(self.payload) + "\n"
                    pl_len = "Payload Length: " + str(self.payload.size) + "\n"
                    return desc + sf_desc + bw_desc + hdr_chk + ih_desc + src_desc + dest_desc + seq_desc + cr_desc + crc_check + pl_len + pl_str

############### END LORA.PY #########################


############### START LORA_HIGHER_LEVEL.PY #########################

def packet_receiver(pkt_queue: mp.Queue, packets, sf, filterID=None, tmt=None):
    print("Started receiver for sf", sf, "and timeout", tmt)
    while True:
        try:
            pkt = pkt_queue.get(timeout=tmt)
        except queue.Empty:
            print("Received nothing, packet_receiver")
            return 
        if isinstance(pkt, LoRaPacket):
            print(f"####### {filterID} {pkt.dst} #######")
            if filterID == 0 or filterID == pkt.dst:
                print(f"####### Received packet on sf {sf} {pkt.payload} #######")
                packets.put_nowait((sf,pkt))
                return

def buffered_packet_receiver(pkt_queue, packets_map, sf):
    print("Started receiver for sf", sf)
    while True:
        pkt = pkt_queue.get()
        if isinstance(pkt, LoRaPacket):
            print(f"####### Received packet on sf {sf} {pkt.payload} #######")
            if packets_map[str(sf)][str(pkt.dst)] == None:
                packets_map[str(sf)][str(pkt.dst)] = mp.Queue(0)
            packets_map[str(sf)][str(pkt.dst)].put_nowait((sf,pkt))

def LoRaBufferedBuilder(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate, rx_ch_ID, tx_ch_ID, spreading_factor):
    lora_radio = lora_transceiver(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate,
                                            rx_ch_ID, tx_ch_ID)
    
    rx_queues = lora_radio.rx_start([spreading_factor])
    tx_queue = lora_radio.tx_start(1)
    return (LoRaSender(lora_radio, tx_queue, spreading_factor, bandwidth), LoRaBufferedReceiver(lora_radio, rx_queues, [spreading_factor]))


class LoRaSender:
    def __init__(self, radio, tx_queue, spreading_factor, bandwidth):
        self.lora_radio = radio
        self.spreading_factor = spreading_factor
        self.tx_queue = tx_queue
        self.bandwidth = bandwidth

    def send_radio(self, data_array, src_id, dest_id):
        CR = 1
        data_array = np.array(data_array, dtype=np.uint8)
        pack = LoRaPacket(data_array, src_id, dest_id, seqn=0, hdr_ok=1, has_crc=1, crc_ok=1,
                                            cr=CR, ih=0, SF=self.spreading_factor, BW=self.bandwidth, rssi=0, snr=0)

        #data = pack_lora_data(data_array, self.spreading_factor, self.bandwidth, pack_size, self.id, dest_id, False, CR)
        self.tx_queue.put_nowait(pack)
        print("Put data in the queue - 2")
        return

class LoRaBufferedReceiver:
    def __init__(self, radio, rx_queues, sf_list):
        self.lora_radio = radio
        self.rx_queues = rx_queues
        self.sf_list = sf_list
        self.packets_map = {}

        for i in range(len(sf_list)):
            self.packets_map[str(sf_list[i])] = {}
            for j in range(0,256):
                self.packets_map[str(sf_list[i])][str(j)] = mp.Queue(0)
            child = threading.Thread(target=buffered_packet_receiver, args=(self.rx_queues[i], self.packets_map, sf_list[i]))
            child.start()
        print("After creating packet_receiver")

    def recv_radio(self, sf_recv_list, d_id, tmt=None):
        packets = []

        for sf in sf_recv_list:
            if str(sf) not in self.packets_map:
                self.packets_map[str(sf)] = {}
                #for j in range(0,256):
                #    self.packets_map[str(sf)][str(j)] = mp.Queue(0)

        for sf in sf_recv_list:
            if str(d_id) not in self.packets_map[str(sf)]:
                self.packets_map[str(sf)][str(d_id)] = mp.Queue(0)
            q = self.packets_map[str(sf)][str(d_id)]
            try:
                p = q.get(timeout=tmt)
                packets.append(p)
            except queue.Empty:
                pass
        return packets

def receiver_routine():
    address = "192.168.40.2"
    rx_gain = 10
    tx_gain = 20
    bandwidth = 125000
    #center_freq = 1e9
    sample_rate = 1e6

    rx_freq = 1010e6  # Hz
    tx_freq = 990e6  # Hz
    tx_ch_ID = 1
    rx_ch_ID = 0

    sf = 7
    dev_id = 0

    (_, receiver) = LoRaBufferedBuilder(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate, rx_ch_ID, tx_ch_ID, sf)
    while True:
        packets = receiver.recv_radio([7], 5)
        print("packets", packets)
        try:
            for pack in packets:
                print(pack)
        except queue.Empty:
            print("No packet received")

def transmitter_routine():
    address = "192.168.40.2"
    rx_gain = 10
    tx_gain = 20
    bandwidth = 125000
    #center_freq = 1e9
    sample_rate = 1e6

    rx_freq = 990e6  # Hz
    tx_freq = 1010e6  # Hz
    tx_ch_ID = 0
    rx_ch_ID = 1

    sf = 7
    dev_id = 0

    (sender, _) = LoRaBufferedBuilder(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate, rx_ch_ID, tx_ch_ID, sf)
    
    for i in range(0,10):
        sender.send_radio([i, i, i, i, i], 0, 0)
        sleep(10)

def main():
    if sys.argv[1] == 'tx':
        transmitter_routine()
    elif sys.argv[1] == 'rx':
        receiver_routine()
            
if __name__ == "__main__":
    main()


############### END LORA_HIGHER_LEVEL.PY #########################


'''

def LoRaBuilder(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate, rx_ch_ID, tx_ch_ID, d_id, spreading_factor):
    lora_radio = lora_transceiver(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate,
                                            rx_ch_ID, tx_ch_ID)
    
    rx_queues = lora_radio.rx_start([spreading_factor])
    tx_queue = lora_radio.tx_start(1)
    
    #receiving = True
    #transmitting = True

    return (LoRaSender(lora_radio, tx_queue, d_id, spreading_factor, bandwidth), LoRaReceiver(lora_radio, rx_queues, d_id))


class LoRaSender:
    def __init__(self, radio, tx_queue, d_id, spreading_factor, bandwidth):
        self.lora_radio = radio
        self.spreading_factor = spreading_factor
        self.tx_queue = tx_queue
        self.bandwidth = bandwidth
        self.id = d_id

    def send_radio(self, data_array, dest_id):
        CR = 1
        data_array = np.array(data_array, dtype=np.uint8)
        pack = LoRaPacket(data_array, self.id, dest_id, seqn=0, hdr_ok=1, has_crc=1, crc_ok=1,
                                            cr=CR, ih=0, SF=self.spreading_factor, BW=self.bandwidth)

        #data = pack_lora_data(data_array, self.spreading_factor, self.bandwidth, pack_size, self.id, dest_id, False, CR)
        self.tx_queue.put_nowait(pack)
        print("Put data in the queue - 2")
        #sleep(1)
        return

class LoRaReceiver:
    def __init__(self, radio, rx_queues, d_id):
        self.lora_radio = radio
        self.rx_queues = rx_queues
        self.id = d_id

    def recv_radio(self, sf_list, timeout=None):
        rx_listeners = []
        packets = mp.Queue(0)
        packets_list = []

        #if not self.receiving:
        #    self.rx_queues = self.lora_radio.rx_start(sf_list)
        #    self.receiving = True
        #    sleep(2)

        for i in range(len(sf_list)):
            child = threading.Thread(target=packet_receiver, args=(self.rx_queues[i], packets, sf_list[i], self.id, timeout))
            child.start()
            rx_listeners.append(child)

        print("After creating packet_receiver")

        for listener in rx_listeners:
            listener.join()
            try:
                packets_list.append(packets.get(timeout=1))
            except mp.TimeoutError:
                print("Empty queue - recv_radio")
        print(packets_list)
        return packets_list



class LoRaRadio:
    def __init__(self, address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate, rx_ch_ID, tx_ch_ID, d_id, spreading_factor):
        self.address = address
        self.rx_gain = rx_gain
        self.tx_gain = tx_gain
        self.bandwidth = bandwidth
        self.rx_freq = rx_freq
        self.tx_freq = tx_freq
        self.sample_rate = sample_rate
        self.rx_ch_ID = rx_ch_ID
        self.tx_ch_ID = tx_ch_ID
        self.id = d_id
        self.spreading_factor = spreading_factor
        self.transmitting = False
        self.receiving = False

        self.lora_radio = lora_transceiver(address, rx_gain, tx_gain, bandwidth, rx_freq, tx_freq, sample_rate,
                                            rx_ch_ID, tx_ch_ID)
        self.tx_queue = None
        self.rx_queues = None
        
        self.rx_queues = self.lora_radio.rx_start([spreading_factor])
        self.receiving = True

        self.tx_queue = self.lora_radio.tx_start(1)
        self.transmitting = True
        sleep(2)

    def to_sender_receiver(self):
        return (LoRaSender(self.tx_queue, self.id, self.spreading_factor, self.bandwidth), LoRaReceiver(self.rx_queues, self.id))
  
    def send_radio(self, data_array, dest_id):
        #print(self)
        #print("data_array:", str(data_array), "dest_id: ", str(dest_id))
        #sleep_time = 1 #TODO capire a che serve e cosa cambia se metto 0 o se forse è il tempo di trasmissione (?)
        #pack_size = 250

        #if not self.transmitting:
        #    self.tx_queue = self.lora_radio.tx_start(sleep_time)
        #    self.transmitting = True
        #    sleep(2)

        CR = 1
        data_array = np.array(data_array, dtype=np.uint8)
        pack = LoRaPacket(data_array, self.id, dest_id, seqn=0, hdr_ok=1, has_crc=1, crc_ok=1,
                                            cr=CR, ih=0, SF=self.spreading_factor, BW=self.bandwidth)

        #data = pack_lora_data(data_array, self.spreading_factor, self.bandwidth, pack_size, self.id, dest_id, False, CR)
        self.tx_queue.put_nowait(pack)
        print("Put data in the queue")
        return

    def recv_radio(self, sf_list, timeout=None):
        rx_listeners = []
        packets = mp.Queue(0)
        packets_list = []

        #if not self.receiving:
        #    self.rx_queues = self.lora_radio.rx_start(sf_list)
        #    self.receiving = True
        #    sleep(2)

        for i in range(len(sf_list)):
            child = threading.Thread(target=packet_receiver, args=(self.rx_queues[i], packets, sf_list[i], self.id, timeout))
            child.start()
            rx_listeners.append(child)

        print("After creating packet_receiver")


        for listener in rx_listeners:
            listener.join()
            try:
                packets_list.append(packets.get_nowait())
            except queue.Empty:
                print("Empty queue")
        return packets_list

    def tx_stop(self):
        if self.transmitting:
            self.lora_radio.tx_stop()
            self.transmitting = False

    def rx_pause(self):
        if self.receiving:
            self.lora_radio.rx_pause()
            self.receiving = False
    
    def rx_resume(self):
        if self.receiving:
            self.lora_radio.rx_resume()
            self.receiving = True

    def rx_stop(self):
        if self.receiving:
            self.lora_radio.rx_stop()
            self.receiving = False

    def stop(self):
        if self.receiving:
            self.lora_radio.rx_stop()
            self.receiving = False
        if self.transmitting:
            self.lora_radio.tx_stop()
            self.transmitting = False

    def __repr__(self) -> str:
        s = ""
        s += f"LoRaRadio.address: {self.address}\n"
        s += f"LoRaRadio.rx_gain: {self.rx_gain}\n"
        s += f"LoRaRadio.tx_gain: {self.tx_gain}\n"
        s += f"LoRaRadio.bandwidth: {self.bandwidth}\n"
        s += f"LoRaRadio.rx_freq: {self.rx_freq}\n"
        s += f"LoRaRadio.tx_freq: {self.tx_freq}\n"
        s += f"LoRaRadio.sample_rate: {self.sample_rate}\n"
        s += f"LoRaRadio.rx_ch_ID: {self.rx_ch_ID}\n"
        s += f"LoRaRadio.tx_ch_ID: {self.tx_ch_ID}\n"
        s += f"LoRaRadio.id: {self.id}\n"
        s += f"LoRaRadio.spreading_factor: {self.spreading_factor}\n"
        return s
'''
