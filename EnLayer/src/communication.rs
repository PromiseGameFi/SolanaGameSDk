use tokio::net::{TcpListener, TcpStream};
use tonic::{transport::Server, Request, Response, Status};
use std::net::SocketAddr;

#[derive(Debug)]
pub struct Communication {
    address: SocketAddr,
    peers: HashMap<u32, SocketAddr>,
}

impl Communication {
    pub async fn new(address: SocketAddr, peers: HashMap<u32, SocketAddr>) -> Self {
        Self {
            address,
            peers,
        }
    }
    
    pub async fn send_share(&self, party_id: u32, share: Scalar) -> Result<(), CommError> {
        let peer_addr = self.peers.get(&party_id)
            .ok_or(CommError::PeerNotFound(party_id))?;
            
        let stream = TcpStream::connect(peer_addr).await?;
        // Implement secure message sending with encryption
        // Use authenticated encryption (e.g., AES-GCM)
        Ok(())
    }
    
    pub async fn start_server(&self) -> Result<(), CommError> {
        let listener = TcpListener::bind(self.address).await?;
        
        loop {
            let (socket, _) = listener.accept().await?;
            // Handle incoming connections and messages
            tokio::spawn(async move {
                Self::handle_connection(socket).await
            });
        }
    }
}