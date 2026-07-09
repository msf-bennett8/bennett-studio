/**
 * Bennett Studio P2P WebRTC Connection
 * Uses data channels for SQL query transport
 * Falls back to relay if P2P fails
 */

interface IceCandidate {
  candidate_type: 'Host' | 'ServerReflexive' | 'Relay';
  address: string;
  protocol: 'udp';
  priority: number;
}

interface IceCandidates {
  candidates: IceCandidate[];
  generated_at: string;
}

interface P2PMessage {
  type: string;
  [key: string]: any;
}

export type P2PState = 'idle' | 'connecting' | 'connected' | 'disconnected' | 'error';

/** Custom error indicating P2P should fallback to relay */
export class P2PRelayFallbackError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'P2PRelayFallbackError';
  }
}

export class P2PConnection {
  private pc: RTCPeerConnection | null = null;
  private dataChannel: RTCDataChannel | null = null;
  private messageQueue: Map<string, { resolve: (value: any) => void; reject: (reason: any) => void }> = new Map();
  private msgId = 0;
  private connected = false;
  private state: P2PState = 'idle';
  private stateListeners: Set<(state: P2PState) => void> = new Set();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 3;

  /**
   * Subscribe to P2P connection state changes
   */
  onStateChange(listener: (state: P2PState) => void): () => void {
    this.stateListeners.add(listener);
    listener(this.state);
    return () => this.stateListeners.delete(listener);
  }

  private setState(state: P2PState): void {
    this.state = state;
    for (const listener of this.stateListeners) {
      listener(state);
    }
  }

  getState(): P2PState {
    return this.state;
  }

  /**
   * Establish P2P connection using ICE candidates from JWT
   */
  async connect(iceB64: string, roomCode: string, firebaseUrl: string): Promise<void> {
    this.setState('connecting');
    this.reconnectAttempts = 0;
    // Decode ICE candidates
    let ice: IceCandidates;
    try {
      ice = JSON.parse(atob(iceB64));
    } catch {
      throw new Error('Invalid ICE candidates format');
    }

    // Create peer connection with STUN
    this.pc = new RTCPeerConnection({
      iceServers: [
        { urls: 'stun:stun.l.google.com:19302' },
        { urls: 'stun:stun1.l.google.com:19302' },
      ],
    });

    // Create data channel for SQL queries
    this.dataChannel = this.pc.createDataChannel('sql', {
      ordered: true,
      maxRetransmits: 3,
    });

    this.setupDataChannelHandlers();

    // Handle ICE candidates
    const iceCandidates: RTCIceCandidateInit[] = [];
    this.pc.onicecandidate = (event) => {
      if (event.candidate) {
        iceCandidates.push(event.candidate.toJSON());
      }
    };

    // Create offer
    const offer = await this.pc.createOffer();
    await this.pc.setLocalDescription(offer);

    // Wait for ICE gathering (with timeout)
    await this.gatherIceComplete(this.pc, 5000);

    // Exchange via Firebase signaling
    await this.exchangeSignaling(offer, iceCandidates, roomCode, firebaseUrl);

    this.connected = true;
  }

  private setupDataChannelHandlers(): void {
    if (!this.dataChannel) return;

    this.dataChannel.onopen = () => {
      console.log('[BennettP2P] Data channel open');
      this.connected = true;
    };

    this.dataChannel.onclose = () => {
      console.log('[BennettP2P] Data channel closed');
      this.connected = false;
    };

    this.dataChannel.onmessage = (event) => {
      try {
        const msg = JSON.parse(event.data);
        const pending = this.messageQueue.get(msg.requestId);
        if (pending) {
          pending.resolve(msg);
          this.messageQueue.delete(msg.requestId);
        }
      } catch (e) {
        console.error('[BennettP2P] Failed to parse message:', e);
      }
    };

    this.dataChannel.onerror = (err) => {
      console.error('[BennettP2P] Data channel error:', err);
    };
  }

  private async gatherIceComplete(pc: RTCPeerConnection, timeoutMs: number): Promise<void> {
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        console.log('[BennettP2P] ICE gathering timeout, using gathered candidates');
        resolve();
      }, timeoutMs);

      pc.onicegatheringstatechange = () => {
        if (pc.iceGatheringState === 'complete') {
          clearTimeout(timer);
          resolve();
        }
      };
    });
  }

  private async exchangeSignaling(
    offer: RTCSessionDescriptionInit,
    iceCandidates: RTCIceCandidateInit[],
    roomCode: string,
    firebaseUrl: string
  ): Promise<void> {
    // Post offer to Firebase
    const roomUrl = `${firebaseUrl}/rooms/${roomCode}.json`;
    
    await fetch(roomUrl, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        client_offer: { sdp: offer.sdp, type: offer.type },
        client_ice: iceCandidates,
        client_connected: false,
      }),
    });

    // Poll for answer (engine side)
    const answer = await this.pollForAnswer(roomCode, firebaseUrl);
    if (!this.pc) throw new Error('PeerConnection closed during signaling');

    await this.pc.setRemoteDescription(answer);

    // Wait for data channel to open
    await this.waitForDataChannelOpen(10000);
  }

  private async pollForAnswer(roomCode: string, firebaseUrl: string): Promise<RTCSessionDescriptionInit> {
    const roomUrl = `${firebaseUrl}/rooms/${roomCode}.json`;
    const deadline = Date.now() + 30000; // 30s timeout

    while (Date.now() < deadline) {
      try {
        const resp = await fetch(roomUrl);
        if (!resp.ok) {
          await this.sleep(1000);
          continue;
        }

        const room = await resp.json();
        if (room?.engine_answer?.sdp) {
          // Check if engine recommends relay fallback
          if (room.engine_answer.mode === 'relay_fallback') {
            this.setState('error');
            // PHASE G: Instead of throwing, return a special marker that triggers fallback
            throw new P2PRelayFallbackError('Engine recommends relay fallback — P2P not available');
          }
          return {
            type: room.engine_answer.type as RTCSdpType,
            sdp: room.engine_answer.sdp,
          };
        }

        // PHASE G: Check for query result via Firebase (SDK fallback pattern)
        if (room?.engine_answer?.type === 'query_result' && room?.engine_answer?.data) {
          // This is a query result, not a WebRTC answer — handled by caller
          throw new P2PRelayFallbackError('Query answered via Firebase relay');
        }
      } catch (e) {
        // Re-throw relay fallback errors
        if (e instanceof P2PRelayFallbackError) {
          throw e;
        }
        // Continue polling on other errors
      }
      await this.sleep(1000);
    }

    this.setState('error');
    throw new Error('Signaling timeout: engine did not respond');
  }

  private async waitForDataChannelOpen(timeoutMs: number): Promise<void> {
    return new Promise((resolve, reject) => {
      if (this.dataChannel?.readyState === 'open') {
        resolve();
        return;
      }

      const timer = setTimeout(() => {
        reject(new Error('Data channel open timeout'));
      }, timeoutMs);

      const checkInterval = setInterval(() => {
        if (this.dataChannel?.readyState === 'open') {
          clearTimeout(timer);
          clearInterval(checkInterval);
          resolve();
        }
      }, 100);
    });
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(r => setTimeout(r, ms));
  }

  /**
   * Send message via data channel and wait for response
   */
  async send(payload: P2PMessage): Promise<any> {
    if (!this.dataChannel || this.dataChannel.readyState !== 'open') {
      throw new Error('P2P data channel not open');
    }

    const requestId = `msg-${++this.msgId}`;
    const message = { ...payload, requestId };

    return new Promise((resolve, reject) => {
      // Set timeout
      const timeout = setTimeout(() => {
        if (this.messageQueue.has(requestId)) {
          this.messageQueue.delete(requestId);
          reject(new Error('P2P query timeout (30s)'));
        }
      }, 30000);

      // Store pending
      this.messageQueue.set(requestId, {
        resolve: (value) => {
          clearTimeout(timeout);
          resolve(value);
        },
        reject: (reason) => {
          clearTimeout(timeout);
          reject(reason);
        },
      });

      // Send
      try {
        this.dataChannel!.send(JSON.stringify(message));
      } catch (e) {
        this.messageQueue.delete(requestId);
        clearTimeout(timeout);
        reject(e);
      }
    });
  }

  isConnected(): boolean {
    return this.connected && this.dataChannel?.readyState === 'open';
  }

  close(): void {
    this.connected = false;
    this.setState('disconnected');
    this.dataChannel?.close();
    this.pc?.close();
    this.dataChannel = null;
    this.pc = null;

    // Reject all pending messages
    for (const [, pending] of this.messageQueue) {
      pending.reject(new Error('P2P connection closed'));
    }
    this.messageQueue.clear();
  }
}