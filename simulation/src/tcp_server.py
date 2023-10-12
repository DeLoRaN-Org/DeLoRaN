import socket

# create a TCP socket
server_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)

# bind the socket to a specific IP address and port number
server_address = ('localhost', 9999)
server_socket.bind(server_address)

# listen for incoming connections (the argument is the maximum number of queued connections)
server_socket.listen(5)

# wait for a client to connect
print('Waiting for a client to connect...')
while True:
    client_socket, client_address = server_socket.accept()
    print('Client connected:', client_address)
    # receive data from the client
    #data = client_socket.recv(1024)
    #print('Received data:', data.decode())
    ## send a response to the client
    #response = 'Hello, client!'
    #client_socket.send(response.encode())

# close the sockets
client_socket.close()
server_socket.close()