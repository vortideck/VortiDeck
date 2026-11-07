# VoiceFlow Telephony Plugin for VortiDeck

> A service plugin that brings Asterisk PBX telephony management, speech-to-text transcription, and voice-activated dialing to VortiDeck flows.

## Overview

The VoiceFlow plugin transforms the [ari-stasi-server](https://github.com/your-repo/ari-stasi-server) telephony system into a collection of VortiDeck flow nodes. This enables visual programming of complex telephony workflows including IVR systems, call routing, voice-activated dialing, and real-time transcription.

---

## System Architecture

```mermaid
graph TB
    subgraph VortiDeck["VortiDeck Application"]
        FE[Flow Editor UI]
        FX[Flow Executor]
        PM[Plugin Manager]
    end

    subgraph VoiceFlowPlugin["VoiceFlow Service Plugin"]
        direction TB
        BG[Background Service]
        NP[Node Provider]

        subgraph Services["Core Services"]
            ARI[ARI Controller]
            RTP[RTP UDP Server]
            STT[Speech Provider]
        end

        subgraph Nodes["Flow Nodes"]
            TN[Trigger Nodes]
            AN[Action Nodes]
            LN[Logic Nodes]
        end
    end

    subgraph External["External Systems"]
        PBX[Asterisk PBX]
        GSTT[Google Speech API]
        AWS[AWS Transcribe]
        WSP[Whisper Self-Hosted]
    end

    FE --> FX
    FX <--> PM
    PM <--> NP
    NP --> Nodes
    BG --> Services

    ARI <--> PBX
    RTP <--> PBX
    STT <--> GSTT
    STT <--> AWS
    STT <--> WSP

    Services --> TN
    AN --> Services
```

---

## Plugin Manifest

```json
{
  "id": "voiceflow:telephony",
  "name": "VoiceFlow Telephony",
  "version": "1.0.0",
  "author": "VortiDeck Team",
  "description": "Asterisk PBX telephony management with speech recognition",
  "vortideck_version": ">=0.1.0",
  "schema_version": "1.0",
  "plugin_type": "service",
  "permissions": [
    "network",
    "storage",
    "background_service"
  ],
  "service": {
    "entry": "service/index.js",
    "auto_start": true
  },
  "configuration": {
    "pbx_ip": { "type": "string", "required": true },
    "pbx_username": { "type": "string", "required": true },
    "pbx_password": { "type": "string", "required": true, "secret": true },
    "speech_provider": { "type": "enum", "values": ["google", "aws", "whisper"], "default": "google" },
    "speech_language": { "type": "string", "default": "en-US" },
    "rtp_port": { "type": "number", "default": 8000 }
  }
}
```

---

## Node Categories

### Trigger Nodes

```mermaid
graph LR
    subgraph Triggers["Event Triggers"]
        IC[IncomingCall]
        OC[OutgoingCallAnswered]
        CE[CallEnded]
        TR[TranscriptionReceived]
        DT[DTMFReceived]
        BR[BridgeEvent]
    end

    subgraph Outputs["Event Data"]
        IC --> |channel_id, caller_number, caller_name| O1[Flow Start]
        OC --> |channel_id, recipient, call_id| O2[Flow Start]
        CE --> |channel_id, duration, end_reason| O3[Flow Start]
        TR --> |channel_id, transcript, is_final, confidence| O4[Flow Start]
        DT --> |channel_id, digit, timestamp| O5[Flow Start]
        BR --> |bridge_id, event_type, channels| O6[Flow Start]
    end
```

### Action Nodes

```mermaid
graph TB
    subgraph CallControl["Call Control"]
        AC[AnswerCall]
        HC[HangupCall]
        OC[OriginateCall]
        TC[TransferCall]
    end

    subgraph BridgeControl["Bridge Control"]
        CB[CreateBridge]
        DB[DestroyBridge]
        AB[AddToBridge]
        RB[RemoveFromBridge]
    end

    subgraph MediaControl["Media Control"]
        PA[PlayAudio]
        SP[StopPlayback]
        EM[CreateExternalMedia]
        RC[StartRecording]
    end

    subgraph TranscriptionControl["Transcription Control"]
        ST[StartTranscription]
        ET[StopTranscription]
        CS[ConfigureSpeech]
    end
```

### Logic Nodes

```mermaid
graph LR
    subgraph Logic["Logic & Utility"]
        CL[ContactLookup]
        MW[MatchWords]
        CU[CallUsageLog]
        RC[RetryCounter]
        AC[ARIConnect]
        GS[GetChannelState]
    end
```

---

## Complete Node Reference

### Trigger Nodes

#### IncomingCall
Fires when a new call enters the Stasis application.

```mermaid
graph LR
    subgraph IncomingCall["IncomingCall Trigger"]
        direction TB
        E[Event from ARI]

        subgraph Outputs
            O1[channel_id: string]
            O2[caller_number: string]
            O3[caller_name: string]
            O4[dialed_number: string]
            O5[timestamp: datetime]
        end
    end

    E --> Outputs
```

#### TranscriptionReceived
Fires when speech-to-text produces a result.

```mermaid
graph LR
    subgraph TranscriptionReceived["TranscriptionReceived Trigger"]
        direction TB
        E[Event from Speech Provider]

        subgraph Outputs
            O1[channel_id: string]
            O2[transcript: string]
            O3[is_final: boolean]
            O4[confidence: number]
            O5[words: array]
        end
    end

    E --> Outputs
```

---

### Action Nodes

#### OriginateCall
Initiates an outbound call.

```mermaid
graph LR
    subgraph OriginateCall["OriginateCall Action"]
        subgraph Inputs
            I1[recipient: string]
            I2[from_number: string]
            I3[caller_id: string]
            I4[timeout: number]
        end

        subgraph Outputs
            O1[channel_id: string]
            O2[success: boolean]
            O3[error: string?]
        end
    end

    Inputs --> |Execute| Outputs
```

#### PlayAudio
Plays audio to a channel.

```mermaid
graph LR
    subgraph PlayAudio["PlayAudio Action"]
        subgraph Inputs
            I1[channel_id: string]
            I2[audio_url: string]
            I3[wait_for_finish: boolean]
        end

        subgraph Outputs
            O1[playback_id: string]
            O2[finished: boolean]
            O3[duration_ms: number]
        end
    end

    Inputs --> |Execute| Outputs
```

#### CreateExternalMedia
Creates an external media channel for RTP streaming (used for transcription).

```mermaid
graph LR
    subgraph CreateExternalMedia["CreateExternalMedia Action"]
        subgraph Inputs
            I1[external_host: string]
            I2[port: number]
            I3[format: enum]
        end

        subgraph Outputs
            O1[channel_id: string]
            O2[rtp_endpoint: string]
            O3[success: boolean]
        end
    end

    Inputs --> |Execute| Outputs
```

---

### Logic Nodes

#### ContactLookup
Searches contacts database by spoken words.

```mermaid
graph LR
    subgraph ContactLookup["ContactLookup Logic"]
        subgraph Inputs
            I1[transcript: string]
            I2[contacts_source: string]
        end

        subgraph Outputs
            O1[found: boolean]
            O2[phone_number: string?]
            O3[contact_name: string?]
            O4[matched_words: array]
        end
    end

    Inputs --> |Process| Outputs
```

#### RetryCounter
Tracks retry attempts with configurable limits.

```mermaid
graph LR
    subgraph RetryCounter["RetryCounter Logic"]
        subgraph Inputs
            I1[max_retries: number]
            I2[increment: boolean]
            I3[reset: boolean]
        end

        subgraph Outputs
            O1[count: number]
            O2[exceeded: boolean]
            O3[should_retry: boolean]
        end
    end

    Inputs --> |Process| Outputs
```

---

## Example Flows

### Flow 1: Voice-Activated Dialing (IVR)

This recreates the main functionality of ari-stasi-server.

```mermaid
flowchart TD
    subgraph Flow["Voice-Activated Dialing Flow"]
        T1[IncomingCall Trigger]
        A1[AnswerCall]
        A2[CreateBridge]
        A3[AddToBridge]
        A4[PlayAudio: welcome]
        A5[CreateExternalMedia]
        A6[AddToBridge: external]
        A7[StartTranscription]
        A8[PlayAudio: beep]

        T2[TranscriptionReceived Trigger]
        L1[ContactLookup]
        B1{Found Contact?}

        L2[RetryCounter]
        B2{Exceeded Retries?}

        A9[PlayAudio: try_again]
        A10[OriginateCall]
        A11[PlayAudio: ringing]
        A12[HangupCall]

        T1 --> A1
        A1 --> A2
        A2 --> A3
        A3 --> A4
        A4 --> A5
        A5 --> A6
        A6 --> A7
        A7 --> A8

        A8 -.-> T2
        T2 --> L1
        L1 --> B1

        B1 -->|Yes| A10
        B1 -->|No| L2

        L2 --> B2
        B2 -->|No| A9
        B2 -->|Yes| A12

        A9 --> A8

        A10 --> A11
    end
```

### Flow 2: Call Recording with Transcription

```mermaid
flowchart TD
    subgraph Flow["Call Recording Flow"]
        T1[IncomingCall]
        A1[AnswerCall]
        A2[CreateBridge]
        A3[AddToBridge]
        A4[CreateExternalMedia]
        A5[StartTranscription]
        A6[StartRecording]

        T2[TranscriptionReceived]
        L1{Is Final?}
        A7[StoreTranscript]

        T3[CallEnded]
        A8[StopRecording]
        A9[StopTranscription]
        A10[CallUsageLog]

        T1 --> A1 --> A2 --> A3 --> A4 --> A5 --> A6

        A6 -.-> T2
        T2 --> L1
        L1 -->|Yes| A7
        L1 -->|No| T2

        A6 -.-> T3
        T3 --> A8 --> A9 --> A10
    end
```

### Flow 3: DTMF Menu Navigation

```mermaid
flowchart TD
    subgraph Flow["DTMF Menu Flow"]
        T1[IncomingCall]
        A1[AnswerCall]
        A2[PlayAudio: menu_options]

        T2[DTMFReceived]
        S1{Digit?}

        A3[PlayAudio: sales_dept]
        A4[TransferCall: sales]

        A5[PlayAudio: support_dept]
        A6[TransferCall: support]

        A7[PlayAudio: invalid_option]

        T1 --> A1 --> A2
        A2 -.-> T2
        T2 --> S1

        S1 -->|1| A3 --> A4
        S1 -->|2| A5 --> A6
        S1 -->|other| A7 --> A2
    end
```

---

## Data Flow Architecture

```mermaid
sequenceDiagram
    participant User as Phone User
    participant PBX as Asterisk PBX
    participant ARI as ARI Controller
    participant RTP as RTP Server
    participant STT as Speech Provider
    participant Flow as Flow Executor
    participant Node as Flow Nodes

    User->>PBX: Incoming Call
    PBX->>ARI: StasisStart Event
    ARI->>Flow: Emit IncomingCall
    Flow->>Node: Execute AnswerCall
    Node->>ARI: Answer Channel
    ARI->>PBX: Answer

    Flow->>Node: Execute CreateExternalMedia
    Node->>ARI: Create External Channel
    ARI->>PBX: External Media
    PBX->>RTP: RTP Audio Stream

    Flow->>Node: Execute StartTranscription
    Node->>STT: Start Streaming
    RTP->>STT: Audio Packets

    STT->>Flow: Transcription Result
    Flow->>Node: Execute ContactLookup
    Node->>Flow: Contact Found

    Flow->>Node: Execute OriginateCall
    Node->>ARI: Originate
    ARI->>PBX: Dial Out
    PBX->>User: Connected
```

---

## Service Lifecycle

```mermaid
stateDiagram-v2
    [*] --> Initializing: Plugin Loaded

    Initializing --> Connecting: Config Valid
    Initializing --> Error: Config Invalid

    Connecting --> Connected: ARI Connected
    Connecting --> Reconnecting: Connection Failed

    Reconnecting --> Connecting: Retry
    Reconnecting --> Error: Max Retries

    Connected --> Running: Bridge Created
    Running --> Connected: All Channels Closed

    Running --> Reconnecting: Connection Lost

    Running --> Stopping: Shutdown Request
    Stopping --> [*]: Cleanup Complete

    Error --> [*]: Fatal Error
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `PBX_IP` | Asterisk PBX IP address | required |
| `ASTERISK_LOGIN` | ARI username | required |
| `ASTERISK_PASSWORD` | ARI password | required |
| `WSS_PORT` | WebSocket server port | 3044 |
| `LISTENER_SERVER` | RTP listener address | 0.0.0.0:8000 |
| `EXTERNAL_HOST` | External host for RTP | required |
| `SPEECH_LANG` | Speech recognition language | en-US |
| `SPEECH_MODEL` | Speech recognition model | default |
| `FROM_NUMBER` | Default caller ID | required |
| `SIP_PROVIDER` | SIP provider trunk | required |

### Contacts Database

```json
{
  "contacts": [
    {
      "name": "John Doe",
      "phone": "+1234567890",
      "words": ["john", "doe", "sales"]
    },
    {
      "name": "Support Team",
      "phone": "+1234567891",
      "words": ["support", "help", "technical"]
    }
  ]
}
```

---

## Speech Provider Comparison

```mermaid
graph TB
    subgraph Providers["Speech-to-Text Providers"]
        subgraph Google["Google Cloud Speech"]
            G1[Streaming API]
            G2[Auto-restart at 25s]
            G3[Speaker diarization]
            G4[High accuracy]
        end

        subgraph AWS["AWS Transcribe"]
            A1[Streaming API]
            A2[Medical vocabulary]
            A3[Custom vocabulary]
            A4[Lower latency]
        end

        subgraph Whisper["Self-Hosted Whisper"]
            W1[Local processing]
            W2[No API costs]
            W3[Privacy focused]
            W4[GPU required]
        end
    end
```

---

## Error Handling

```mermaid
flowchart TD
    subgraph ErrorHandling["Error Handling Strategy"]
        E1[ARI Connection Error]
        E2[Speech API Error]
        E3[Channel Error]
        E4[Bridge Error]

        R1[Reconnect with Backoff]
        R2[Switch Provider / Retry]
        R3[Cleanup & Notify]
        R4[Recreate Bridge]

        E1 --> R1
        E2 --> R2
        E3 --> R3
        E4 --> R4

        R1 --> |Success| S1[Resume]
        R1 --> |Fail| F1[Emit Error Event]

        R2 --> |Success| S1
        R2 --> |Fail| F1

        R3 --> F1
        R4 --> |Success| S1
        R4 --> |Fail| F1
    end
```

---

## Implementation Roadmap

### Phase 1: Core Infrastructure
- [ ] Service plugin scaffold
- [ ] ARI connection management
- [ ] Basic call control nodes (Answer, Hangup, Originate)
- [ ] Bridge management nodes

### Phase 2: Audio & Transcription
- [ ] RTP UDP server integration
- [ ] External media channel support
- [ ] Google Speech provider
- [ ] Transcription trigger node

### Phase 3: Advanced Features
- [ ] Contact lookup logic node
- [ ] DTMF handling
- [ ] Call recording
- [ ] Multiple speech providers (AWS, Whisper)

### Phase 4: Polish & Optimization
- [ ] Error recovery and reconnection
- [ ] Call analytics and logging
- [ ] Web UI for configuration
- [ ] Performance optimization

---

## Related Documentation

- [Plugin Architecture](./plugin-architecture.md)
- Service Plugin Development
- Flow Node API
- [Asterisk ARI Documentation](https://wiki.asterisk.org/wiki/display/AST/Asterisk+REST+Interface)
- [Google Cloud Speech-to-Text](https://cloud.google.com/speech-to-text/docs)
