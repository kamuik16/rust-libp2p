// Copyright 2020 Sigma Prime Pty Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use std::{borrow::Cow, collections::HashMap, sync::Arc, time::Duration};

use libp2p_identity::PeerId;
use libp2p_swarm::StreamProtocol;

use crate::{
    error::ConfigBuilderError,
    protocol::{ProtocolConfig, ProtocolId, FLOODSUB_PROTOCOL},
    types::{Message, MessageId, PeerKind},
    TopicHash,
};

/// The types of message validation that can be employed by gossipsub.
#[derive(Debug, Clone)]
pub enum ValidationMode {
    /// This is the default setting. This requires the message author to be a valid [`PeerId`] and
    /// to be present as well as the sequence number. All messages must have valid signatures.
    ///
    /// NOTE: This setting will reject messages from nodes using
    /// [`crate::behaviour::MessageAuthenticity::Anonymous`] and all messages that do not have
    /// signatures.
    Strict,
    /// This setting permits messages that have no author, sequence number or signature. If any of
    /// these fields exist in the message these are validated.
    Permissive,
    /// This setting requires the author, sequence number and signature fields of a message to be
    /// empty. Any message that contains these fields is considered invalid.
    Anonymous,
    /// This setting does not check the author, sequence number or signature fields of incoming
    /// messages. If these fields contain data, they are simply ignored.
    ///
    /// NOTE: This setting will consider messages with invalid signatures as valid messages.
    None,
}

/// Selector for custom Protocol Id
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Version {
    V1_0,
    V1_1,
}

/// Defines the overall configuration for mesh parameters and max transmit sizes
/// for topics.
#[derive(Default, Clone, PartialEq, Eq)]
pub(crate) struct TopicConfigs {
    pub(crate) topic_mesh_params: HashMap<TopicHash, TopicMeshConfig>,
    pub(crate) default_mesh_params: TopicMeshConfig,
}

/// Defines the mesh network parameters for a given topic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TopicMeshConfig {
    pub mesh_n: usize,
    pub mesh_n_low: usize,
    pub mesh_n_high: usize,
    pub mesh_outbound_min: usize,
}

impl Default for TopicMeshConfig {
    fn default() -> Self {
        Self {
            mesh_n: 6,
            mesh_n_low: 5,
            mesh_n_high: 12,
            mesh_outbound_min: 2,
        }
    }
}

impl std::fmt::Debug for TopicConfigs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("TopicConfigs");
        let _ = builder.field("topic_mesh_params", &self.topic_mesh_params);
        builder.finish()
    }
}

/// Configuration parameters that define the performance of the gossipsub network.
#[derive(Clone)]
pub struct Config {
    protocol: ProtocolConfig,
    history_length: usize,
    history_gossip: usize,
    retain_scores: usize,
    gossip_lazy: usize,
    gossip_factor: f64,
    heartbeat_initial_delay: Duration,
    heartbeat_interval: Duration,
    fanout_ttl: Duration,
    check_explicit_peers_ticks: u64,
    duplicate_cache_time: Duration,
    validate_messages: bool,
    message_id_fn: Arc<dyn Fn(&Message) -> MessageId + Send + Sync + 'static>,
    allow_self_origin: bool,
    do_px: bool,
    prune_peers: usize,
    prune_backoff: Duration,
    unsubscribe_backoff: Duration,
    backoff_slack: u32,
    flood_publish: bool,
    graft_flood_threshold: Duration,
    opportunistic_graft_ticks: u64,
    opportunistic_graft_peers: usize,
    gossip_retransimission: u32,
    max_messages_per_rpc: Option<usize>,
    max_ihave_length: usize,
    max_ihave_messages: usize,
    iwant_followup_time: Duration,
    published_message_ids_cache_time: Duration,
    connection_handler_queue_len: usize,
    connection_handler_publish_duration: Duration,
    connection_handler_forward_duration: Duration,
    idontwant_message_size_threshold: usize,
    idontwant_on_publish: bool,
    topic_configuration: TopicConfigs,
}

impl Config {
    pub(crate) fn protocol_config(&self) -> ProtocolConfig {
        self.protocol.clone()
    }

    // Overlay network parameters.
    /// Number of heartbeats to keep in the `memcache` (default is 5).
    pub fn history_length(&self) -> usize {
        self.history_length
    }

    /// Number of past heartbeats to gossip about (default is 3).
    pub fn history_gossip(&self) -> usize {
        self.history_gossip
    }

    /// Target number of peers for the mesh network (D in the spec, default is 6).
    pub fn mesh_n(&self) -> usize {
        self.topic_configuration.default_mesh_params.mesh_n
    }

    /// Minimum number of peers in mesh network before adding more (D_lo in the spec, default is 5).
    pub fn mesh_n_low(&self) -> usize {
        self.topic_configuration.default_mesh_params.mesh_n_low
    }

    /// Maximum number of peers in mesh network before removing some (D_high in the spec, default
    /// is 12).
    pub fn mesh_n_high(&self) -> usize {
        self.topic_configuration.default_mesh_params.mesh_n_high
    }

    /// Target number of peers for the mesh network for a given topic (D in the spec, default is 6).
    pub fn mesh_n_for_topic(&self, topic_hash: &TopicHash) -> usize {
        self.topic_configuration
            .topic_mesh_params
            .get(topic_hash)
            .unwrap_or(&self.topic_configuration.default_mesh_params)
            .mesh_n
    }

    /// Minimum number of peers in mesh network for a given topic before adding more (D_lo in the
    /// spec, default is 5).
    pub fn mesh_n_low_for_topic(&self, topic_hash: &TopicHash) -> usize {
        self.topic_configuration
            .topic_mesh_params
            .get(topic_hash)
            .unwrap_or(&self.topic_configuration.default_mesh_params)
            .mesh_n_low
    }

    /// Maximum number of peers in mesh network before removing some (D_high in the spec, default
    /// is 12).
    pub fn mesh_n_high_for_topic(&self, topic_hash: &TopicHash) -> usize {
        self.topic_configuration
            .topic_mesh_params
            .get(topic_hash)
            .unwrap_or(&self.topic_configuration.default_mesh_params)
            .mesh_n_high
    }

    /// Affects how peers are selected when pruning a mesh due to over subscription.
    ///
    ///  At least `retain_scores` of the retained peers will be high-scoring, while the remainder
    /// are  chosen randomly (D_score in the spec, default is 4).
    pub fn retain_scores(&self) -> usize {
        self.retain_scores
    }

    /// Minimum number of peers to emit gossip to during a heartbeat (D_lazy in the spec,
    /// default is 6).
    pub fn gossip_lazy(&self) -> usize {
        self.gossip_lazy
    }

    /// Affects how many peers we will emit gossip to at each heartbeat.
    ///
    /// We will send gossip to `gossip_factor * (total number of non-mesh peers)`, or
    /// `gossip_lazy`, whichever is greater. The default is 0.25.
    pub fn gossip_factor(&self) -> f64 {
        self.gossip_factor
    }

    /// Initial delay in each heartbeat (default is 5 seconds).
    pub fn heartbeat_initial_delay(&self) -> Duration {
        self.heartbeat_initial_delay
    }

    /// Time between each heartbeat (default is 1 second).
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Time to live for fanout peers (default is 60 seconds).
    pub fn fanout_ttl(&self) -> Duration {
        self.fanout_ttl
    }

    /// The number of heartbeat ticks until we recheck the connection to explicit peers and
    /// reconnecting if necessary (default 300).
    pub fn check_explicit_peers_ticks(&self) -> u64 {
        self.check_explicit_peers_ticks
    }

    /// The default global max transmit size for topics.
    pub fn default_max_transmit_size() -> usize {
        65536
    }

    /// The maximum byte size for each gossipsub RPC (default is 65536 bytes).
    ///
    /// This represents the maximum size of the published message. It is additionally wrapped
    /// in a protobuf struct, so the actual wire size may be a bit larger. It must be at least
    /// large enough to support basic control messages. If Peer eXchange is enabled, this
    /// must be large enough to transmit the desired peer information on pruning. It must be at
    /// least 100 bytes. Default is 65536 bytes.
    pub fn max_transmit_size(&self) -> usize {
        self.protocol.default_max_transmit_size
    }

    /// The maximum byte size for each gossipsub RPC for a given topic (default is 65536 bytes).
    ///
    /// This represents the maximum size of the published message. It is additionally wrapped
    /// in a protobuf struct, so the actual wire size may be a bit larger. It must be at least
    /// large enough to support basic control messages. If Peer eXchange is enabled, this
    /// must be large enough to transmit the desired peer information on pruning. It must be at
    /// least 100 bytes. Default is 65536 bytes.
    pub fn max_transmit_size_for_topic(&self, topic: &TopicHash) -> usize {
        self.protocol_config().max_transmit_size_for_topic(topic)
    }

    /// Duplicates are prevented by storing message id's of known messages in an LRU time cache.
    /// This settings sets the time period that messages are stored in the cache. Duplicates can be
    /// received if duplicate messages are sent at a time greater than this setting apart. The
    /// default is 1 minute.
    pub fn duplicate_cache_time(&self) -> Duration {
        self.duplicate_cache_time
    }

    /// When set to `true`, prevents automatic forwarding of all received messages. This setting
    /// allows a user to validate the messages before propagating them to their peers. If set to
    /// true, the user must manually call [`crate::Behaviour::report_message_validation_result()`]
    /// on the behaviour to forward message once validated (default is `false`).
    /// The default is `false`.
    pub fn validate_messages(&self) -> bool {
        self.validate_messages
    }

    /// Determines the level of validation used when receiving messages. See [`ValidationMode`]
    /// for the available types. The default is ValidationMode::Strict.
    pub fn validation_mode(&self) -> &ValidationMode {
        &self.protocol.validation_mode
    }

    /// A user-defined function allowing the user to specify the message id of a gossipsub message.
    /// The default value is to concatenate the source peer id with a sequence number. Setting this
    /// parameter allows the user to address packets arbitrarily. One example is content based
    /// addressing, where this function may be set to `hash(message)`. This would prevent messages
    /// of the same content from being duplicated.
    ///
    /// The function takes a [`Message`] as input and outputs a String to be interpreted as
    /// the message id.
    pub fn message_id(&self, message: &Message) -> MessageId {
        (self.message_id_fn)(message)
    }

    /// By default, gossipsub will reject messages that are sent to us that have the same message
    /// source as we have specified locally. Enabling this, allows these messages and prevents
    /// penalizing the peer that sent us the message. Default is false.
    pub fn allow_self_origin(&self) -> bool {
        self.allow_self_origin
    }

    /// Whether Peer eXchange is enabled; this should be enabled in bootstrappers and other well
    /// connected/trusted nodes. The default is false.
    ///
    /// Note: Peer exchange is not implemented today, see
    /// <https://github.com/libp2p/rust-libp2p/issues/2398>.
    pub fn do_px(&self) -> bool {
        self.do_px
    }

    /// Controls the number of peers to include in prune Peer eXchange.
    /// When we prune a peer that's eligible for PX (has a good score, etc), we will try to
    /// send them signed peer records for up to `prune_peers` other peers that we
    /// know of. It is recommended that this value is larger than `mesh_n_high` so that the pruned
    /// peer can reliably form a full mesh. The default is typically 16 however until signed
    /// records are spec'd this is disabled and set to 0.
    pub fn prune_peers(&self) -> usize {
        self.prune_peers
    }

    /// Controls the backoff time for pruned peers. This is how long
    /// a peer must wait before attempting to graft into our mesh again after being pruned.
    /// When pruning a peer, we send them our value of `prune_backoff` so they know
    /// the minimum time to wait. Peers running older versions may not send a backoff time,
    /// so if we receive a prune message without one, we will wait at least `prune_backoff`
    /// before attempting to re-graft. The default is one minute.
    pub fn prune_backoff(&self) -> Duration {
        self.prune_backoff
    }

    /// Controls the backoff time when unsubscribing from a topic.
    ///
    /// This is how long to wait before resubscribing to the topic. A short backoff period in case
    /// of an unsubscribe event allows reaching a healthy mesh in a more timely manner. The default
    /// is 10 seconds.
    pub fn unsubscribe_backoff(&self) -> Duration {
        self.unsubscribe_backoff
    }

    /// Number of heartbeat slots considered as slack for backoffs. This guarantees that we wait
    /// at least backoff_slack heartbeats after a backoff is over before we try to graft. This
    /// solves problems occurring through high latencies. In particular if
    /// `backoff_slack * heartbeat_interval` is longer than any latencies between processing
    /// prunes on our side and processing prunes on the receiving side this guarantees that we
    /// get not punished for too early grafting. The default is 1.
    pub fn backoff_slack(&self) -> u32 {
        self.backoff_slack
    }

    /// Whether to do flood publishing or not. If enabled newly created messages will always be
    /// sent to all peers that are subscribed to the topic and have a good enough score.
    /// The default is true.
    pub fn flood_publish(&self) -> bool {
        self.flood_publish
    }

    /// If a GRAFT comes before `graft_flood_threshold` has elapsed since the last PRUNE,
    /// then there is an extra score penalty applied to the peer through P7.
    pub fn graft_flood_threshold(&self) -> Duration {
        self.graft_flood_threshold
    }

    /// Minimum number of outbound peers in the mesh network before adding more (D_out in the spec).
    /// This value must be smaller or equal than `mesh_n / 2` and smaller than `mesh_n_low`.
    /// The default is 2.
    pub fn mesh_outbound_min(&self) -> usize {
        self.topic_configuration
            .default_mesh_params
            .mesh_outbound_min
    }

    /// Minimum number of outbound peers in the mesh network before adding more (D_out in the spec).
    /// This value must be smaller or equal than `mesh_n / 2` and smaller than `mesh_n_low`.
    /// The default is 2.
    pub fn mesh_outbound_min_for_topic(&self, topic_hash: &TopicHash) -> usize {
        self.topic_configuration
            .topic_mesh_params
            .get(topic_hash)
            .unwrap_or(&self.topic_configuration.default_mesh_params)
            .mesh_outbound_min
    }

    /// Number of heartbeat ticks that specify the interval in which opportunistic grafting is
    /// applied. Every `opportunistic_graft_ticks` we will attempt to select some high-scoring mesh
    /// peers to replace lower-scoring ones, if the median score of our mesh peers falls below a
    /// threshold (see <https://godoc.org/github.com/libp2p/go-libp2p-pubsub#PeerScoreThresholds>).
    /// The default is 60.
    pub fn opportunistic_graft_ticks(&self) -> u64 {
        self.opportunistic_graft_ticks
    }

    /// Controls how many times we will allow a peer to request the same message id through IWANT
    /// gossip before we start ignoring them. This is designed to prevent peers from spamming us
    /// with requests and wasting our resources. The default is 3.
    pub fn gossip_retransimission(&self) -> u32 {
        self.gossip_retransimission
    }

    /// The maximum number of new peers to graft to during opportunistic grafting. The default is 2.
    pub fn opportunistic_graft_peers(&self) -> usize {
        self.opportunistic_graft_peers
    }

    /// The maximum number of messages we will process in a given RPC. If this is unset, there is
    /// no limit. The default is None.
    pub fn max_messages_per_rpc(&self) -> Option<usize> {
        self.max_messages_per_rpc
    }

    /// The maximum number of messages to include in an IHAVE message.
    /// Also controls the maximum number of IHAVE ids we will accept and request with IWANT from a
    /// peer within a heartbeat, to protect from IHAVE floods. You should adjust this value from the
    /// default if your system is pushing more than 5000 messages in GossipSubHistoryGossip
    /// heartbeats; with the defaults this is 1666 messages/s. The default is 5000.
    pub fn max_ihave_length(&self) -> usize {
        self.max_ihave_length
    }

    /// GossipSubMaxIHaveMessages is the maximum number of IHAVE messages to accept from a peer
    /// within a heartbeat.
    pub fn max_ihave_messages(&self) -> usize {
        self.max_ihave_messages
    }

    /// Time to wait for a message requested through IWANT following an IHAVE advertisement.
    /// If the message is not received within this window, a broken promise is declared and
    /// the router may apply behavioural penalties. The default is 3 seconds.
    pub fn iwant_followup_time(&self) -> Duration {
        self.iwant_followup_time
    }

    /// Enable support for flooodsub peers. Default false.
    pub fn support_floodsub(&self) -> bool {
        self.protocol.protocol_ids.contains(&FLOODSUB_PROTOCOL)
    }

    /// Published message ids time cache duration. The default is 10 seconds.
    pub fn published_message_ids_cache_time(&self) -> Duration {
        self.published_message_ids_cache_time
    }

    /// The max number of messages a `ConnectionHandler` can buffer. The default is 5000.
    pub fn connection_handler_queue_len(&self) -> usize {
        self.connection_handler_queue_len
    }

    /// The duration a message to be published can wait to be sent before it is abandoned. The
    /// default is 5 seconds.
    pub fn publish_queue_duration(&self) -> Duration {
        self.connection_handler_publish_duration
    }

    /// The duration a message to be forwarded can wait to be sent before it is abandoned. The
    /// default is 1s.
    pub fn forward_queue_duration(&self) -> Duration {
        self.connection_handler_forward_duration
    }

    /// The message size threshold for which IDONTWANT messages are sent.
    /// Sending IDONTWANT messages for small messages can have a negative effect to the overall
    /// traffic and CPU load. This acts as a lower bound cutoff for the message size to which
    /// IDONTWANT won't be sent to peers. Only works if the peers support Gossipsub1.2
    /// (see <https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.2.md#idontwant-message>)
    /// default is 1kB
    pub fn idontwant_message_size_threshold(&self) -> usize {
        self.idontwant_message_size_threshold
    }

    /// Send IDONTWANT messages after publishing message on gossip. This is an optimisation
    /// to avoid bandwidth consumption by downloading the published message over gossip.
    /// By default it is false.
    pub fn idontwant_on_publish(&self) -> bool {
        self.idontwant_on_publish
    }
}

impl Default for Config {
    fn default() -> Self {
        // use ConfigBuilder to also validate defaults
        ConfigBuilder::default()
            .build()
            .expect("Default config parameters should be valid parameters")
    }
}

/// The builder struct for constructing a gossipsub configuration.
pub struct ConfigBuilder {
    config: Config,
    invalid_protocol: bool, // This is a bit of a hack to only expose one error to the user.
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        ConfigBuilder {
            config: Config {
                protocol: ProtocolConfig::default(),
                history_length: 5,
                history_gossip: 3,
                retain_scores: 4,
                gossip_lazy: 6, // default to mesh_n
                gossip_factor: 0.25,
                heartbeat_initial_delay: Duration::from_secs(5),
                heartbeat_interval: Duration::from_secs(1),
                fanout_ttl: Duration::from_secs(60),
                check_explicit_peers_ticks: 300,
                duplicate_cache_time: Duration::from_secs(60),
                validate_messages: false,
                message_id_fn: Arc::new(|message| {
                    // default message id is: source + sequence number
                    // NOTE: If either the peer_id or source is not provided, we set to 0;
                    let mut source_string = if let Some(peer_id) = message.source.as_ref() {
                        peer_id.to_base58()
                    } else {
                        PeerId::from_bytes(&[0, 1, 0])
                            .expect("Valid peer id")
                            .to_base58()
                    };
                    source_string
                        .push_str(&message.sequence_number.unwrap_or_default().to_string());
                    MessageId::from(source_string)
                }),
                allow_self_origin: false,
                do_px: false,
                // NOTE: Increasing this currently has little effect until Signed
                // records are implemented.
                prune_peers: 0,
                prune_backoff: Duration::from_secs(60),
                unsubscribe_backoff: Duration::from_secs(10),
                backoff_slack: 1,
                flood_publish: true,
                graft_flood_threshold: Duration::from_secs(10),
                opportunistic_graft_ticks: 60,
                opportunistic_graft_peers: 2,
                gossip_retransimission: 3,
                max_messages_per_rpc: None,
                max_ihave_length: 5000,
                max_ihave_messages: 10,
                iwant_followup_time: Duration::from_secs(3),
                published_message_ids_cache_time: Duration::from_secs(10),
                connection_handler_queue_len: 5000,
                connection_handler_publish_duration: Duration::from_secs(5),
                connection_handler_forward_duration: Duration::from_secs(1),
                idontwant_message_size_threshold: 1000,
                idontwant_on_publish: false,
                topic_configuration: TopicConfigs::default(),
            },
            invalid_protocol: false,
        }
    }
}

impl From<Config> for ConfigBuilder {
    fn from(config: Config) -> Self {
        ConfigBuilder {
            config,
            invalid_protocol: false,
        }
    }
}

impl ConfigBuilder {
    /// The protocol id prefix to negotiate this protocol (default is `/meshsub/1.1.0` and
    /// `/meshsub/1.0.0`).
    pub fn protocol_id_prefix(
        &mut self,
        protocol_id_prefix: impl Into<Cow<'static, str>>,
    ) -> &mut Self {
        let cow = protocol_id_prefix.into();

        match (
            StreamProtocol::try_from_owned(format!("{cow}/1.1.0")),
            StreamProtocol::try_from_owned(format!("{cow}/1.0.0")),
        ) {
            (Ok(p1), Ok(p2)) => {
                self.config.protocol.protocol_ids = vec![
                    ProtocolId {
                        protocol: p1,
                        kind: PeerKind::Gossipsubv1_1,
                    },
                    ProtocolId {
                        protocol: p2,
                        kind: PeerKind::Gossipsub,
                    },
                ]
            }
            _ => {
                self.invalid_protocol = true;
            }
        }

        self
    }

    /// The full protocol id to negotiate this protocol (does not append `/1.0.0` or `/1.1.0`).
    pub fn protocol_id(
        &mut self,
        protocol_id: impl Into<Cow<'static, str>>,
        custom_id_version: Version,
    ) -> &mut Self {
        let cow = protocol_id.into();

        match StreamProtocol::try_from_owned(cow.to_string()) {
            Ok(protocol) => {
                self.config.protocol.protocol_ids = vec![ProtocolId {
                    protocol,
                    kind: match custom_id_version {
                        Version::V1_1 => PeerKind::Gossipsubv1_1,
                        Version::V1_0 => PeerKind::Gossipsub,
                    },
                }]
            }
            _ => {
                self.invalid_protocol = true;
            }
        }

        self
    }

    /// Number of heartbeats to keep in the `memcache` (default is 5).
    pub fn history_length(&mut self, history_length: usize) -> &mut Self {
        self.config.history_length = history_length;
        self
    }

    /// Number of past heartbeats to gossip about (default is 3).
    pub fn history_gossip(&mut self, history_gossip: usize) -> &mut Self {
        self.config.history_gossip = history_gossip;
        self
    }

    /// Target number of peers for the mesh network (D in the spec, default is 6).
    pub fn mesh_n(&mut self, mesh_n: usize) -> &mut Self {
        self.config.topic_configuration.default_mesh_params.mesh_n = mesh_n;
        self
    }

    /// Target number of peers for the mesh network for a topic (D in the spec, default is 6).
    pub fn mesh_n_for_topic(&mut self, mesh_n: usize, topic_hash: TopicHash) -> &mut Self {
        self.config
            .topic_configuration
            .topic_mesh_params
            .entry(topic_hash)
            .and_modify(|existing_config| {
                existing_config.mesh_n = mesh_n;
            })
            .or_insert_with(|| TopicMeshConfig {
                mesh_n,
                ..TopicMeshConfig::default()
            });
        self
    }

    /// Maximum number of peers in mesh network before removing some (D_high in the spec, default
    /// is 12).
    pub fn mesh_n_high(&mut self, mesh_n_high: usize) -> &mut Self {
        self.config
            .topic_configuration
            .default_mesh_params
            .mesh_n_high = mesh_n_high;
        self
    }

    /// Maximum number of peers in mesh network for a topic before removing some (D_high in the
    /// spec, default is 12).
    pub fn mesh_n_high_for_topic(
        &mut self,
        mesh_n_high: usize,
        topic_hash: TopicHash,
    ) -> &mut Self {
        self.config
            .topic_configuration
            .topic_mesh_params
            .entry(topic_hash)
            .and_modify(|existing_config| {
                existing_config.mesh_n_high = mesh_n_high;
            })
            .or_insert_with(|| TopicMeshConfig {
                mesh_n_high,
                ..TopicMeshConfig::default()
            });
        self
    }

    /// Minimum number of peers in mesh network before adding more (D_lo in the spec, default is 4).
    pub fn mesh_n_low(&mut self, mesh_n_low: usize) -> &mut Self {
        self.config
            .topic_configuration
            .default_mesh_params
            .mesh_n_low = mesh_n_low;
        self
    }

    /// Minimum number of peers in mesh network for a topic before adding more (D_lo in the spec,
    /// default is 4).
    pub fn mesh_n_low_for_topic(&mut self, mesh_n_low: usize, topic_hash: TopicHash) -> &mut Self {
        self.config
            .topic_configuration
            .topic_mesh_params
            .entry(topic_hash)
            .and_modify(|existing_config| {
                existing_config.mesh_n_low = mesh_n_low;
            })
            .or_insert_with(|| TopicMeshConfig {
                mesh_n_low,
                ..TopicMeshConfig::default()
            });
        self
    }

    /// Affects how peers are selected when pruning a mesh due to over subscription.
    ///
    /// At least [`Self::retain_scores`] of the retained peers will be high-scoring, while the
    /// remainder are chosen randomly (D_score in the spec, default is 4).
    pub fn retain_scores(&mut self, retain_scores: usize) -> &mut Self {
        self.config.retain_scores = retain_scores;
        self
    }

    /// Minimum number of peers to emit gossip to during a heartbeat (D_lazy in the spec,
    /// default is 6).
    pub fn gossip_lazy(&mut self, gossip_lazy: usize) -> &mut Self {
        self.config.gossip_lazy = gossip_lazy;
        self
    }

    /// Affects how many peers we will emit gossip to at each heartbeat.
    ///
    /// We will send gossip to `gossip_factor * (total number of non-mesh peers)`, or
    /// `gossip_lazy`, whichever is greater. The default is 0.25.
    pub fn gossip_factor(&mut self, gossip_factor: f64) -> &mut Self {
        self.config.gossip_factor = gossip_factor;
        self
    }

    /// Initial delay in each heartbeat (default is 5 seconds).
    pub fn heartbeat_initial_delay(&mut self, heartbeat_initial_delay: Duration) -> &mut Self {
        self.config.heartbeat_initial_delay = heartbeat_initial_delay;
        self
    }

    /// Time between each heartbeat (default is 1 second).
    pub fn heartbeat_interval(&mut self, heartbeat_interval: Duration) -> &mut Self {
        self.config.heartbeat_interval = heartbeat_interval;
        self
    }

    /// The number of heartbeat ticks until we recheck the connection to explicit peers and
    /// reconnecting if necessary (default 300).
    pub fn check_explicit_peers_ticks(&mut self, check_explicit_peers_ticks: u64) -> &mut Self {
        self.config.check_explicit_peers_ticks = check_explicit_peers_ticks;
        self
    }

    /// Time to live for fanout peers (default is 60 seconds).
    pub fn fanout_ttl(&mut self, fanout_ttl: Duration) -> &mut Self {
        self.config.fanout_ttl = fanout_ttl;
        self
    }

    /// The maximum byte size for each gossip (default is 2048 bytes).
    pub fn max_transmit_size(&mut self, max_transmit_size: usize) -> &mut Self {
        self.config.protocol.default_max_transmit_size = max_transmit_size;
        self
    }

    /// The maximum byte size for each gossip for a given topic. (default is 2048 bytes).
    pub fn max_transmit_size_for_topic(
        &mut self,
        max_transmit_size: usize,
        topic: TopicHash,
    ) -> &mut Self {
        self.config
            .protocol
            .max_transmit_sizes
            .insert(topic, max_transmit_size);
        self
    }

    /// Duplicates are prevented by storing message id's of known messages in an LRU time cache.
    /// This settings sets the time period that messages are stored in the cache. Duplicates can be
    /// received if duplicate messages are sent at a time greater than this setting apart. The
    /// default is 1 minute.
    pub fn duplicate_cache_time(&mut self, cache_size: Duration) -> &mut Self {
        self.config.duplicate_cache_time = cache_size;
        self
    }

    /// When set, prevents automatic forwarding of all received messages. This setting
    /// allows a user to validate the messages before propagating them to their peers. If set,
    /// the user must manually call [`crate::Behaviour::report_message_validation_result()`] on the
    /// behaviour to forward a message once validated.
    pub fn validate_messages(&mut self) -> &mut Self {
        self.config.validate_messages = true;
        self
    }

    /// Determines the level of validation used when receiving messages. See [`ValidationMode`]
    /// for the available types. The default is ValidationMode::Strict.
    pub fn validation_mode(&mut self, validation_mode: ValidationMode) -> &mut Self {
        self.config.protocol.validation_mode = validation_mode;
        self
    }

    /// A user-defined function allowing the user to specify the message id of a gossipsub message.
    /// The default value is to concatenate the source peer id with a sequence number. Setting this
    /// parameter allows the user to address packets arbitrarily. One example is content based
    /// addressing, where this function may be set to `hash(message)`. This would prevent messages
    /// of the same content from being duplicated.
    ///
    /// The function takes a [`Message`] as input and outputs a String to be
    /// interpreted as the message id.
    pub fn message_id_fn<F>(&mut self, id_fn: F) -> &mut Self
    where
        F: Fn(&Message) -> MessageId + Send + Sync + 'static,
    {
        self.config.message_id_fn = Arc::new(id_fn);
        self
    }

    /// Enables Peer eXchange. This should be enabled in bootstrappers and other well
    /// connected/trusted nodes. The default is false.
    ///
    /// Note: Peer exchange is not implemented today, see
    /// <https://github.com/libp2p/rust-libp2p/issues/2398>.
    pub fn do_px(&mut self) -> &mut Self {
        self.config.do_px = true;
        self
    }

    /// Controls the number of peers to include in prune Peer eXchange.
    ///
    /// When we prune a peer that's eligible for PX (has a good score, etc), we will try to
    /// send them signed peer records for up to [`Self::prune_peers] other peers that we
    /// know of. It is recommended that this value is larger than [`Self::mesh_n_high`] so that the
    /// pruned peer can reliably form a full mesh. The default is 16.
    pub fn prune_peers(&mut self, prune_peers: usize) -> &mut Self {
        self.config.prune_peers = prune_peers;
        self
    }

    /// Controls the backoff time for pruned peers. This is how long
    /// a peer must wait before attempting to graft into our mesh again after being pruned.
    /// When pruning a peer, we send them our value of [`Self::prune_backoff`] so they know
    /// the minimum time to wait. Peers running older versions may not send a backoff time,
    /// so if we receive a prune message without one, we will wait at least [`Self::prune_backoff`]
    /// before attempting to re-graft. The default is one minute.
    pub fn prune_backoff(&mut self, prune_backoff: Duration) -> &mut Self {
        self.config.prune_backoff = prune_backoff;
        self
    }

    /// Controls the backoff time when unsubscribing from a topic.
    ///
    /// This is how long to wait before resubscribing to the topic. A short backoff period in case
    /// of an unsubscribe event allows reaching a healthy mesh in a more timely manner. The default
    /// is 10 seconds.
    pub fn unsubscribe_backoff(&mut self, unsubscribe_backoff: u64) -> &mut Self {
        self.config.unsubscribe_backoff = Duration::from_secs(unsubscribe_backoff);
        self
    }

    /// Number of heartbeat slots considered as slack for backoffs. This guarantees that we wait
    /// at least backoff_slack heartbeats after a backoff is over before we try to graft. This
    /// solves problems occurring through high latencies. In particular if
    /// `backoff_slack * heartbeat_interval` is longer than any latencies between processing
    /// prunes on our side and processing prunes on the receiving side this guarantees that we
    /// get not punished for too early grafting. The default is 1.
    pub fn backoff_slack(&mut self, backoff_slack: u32) -> &mut Self {
        self.config.backoff_slack = backoff_slack;
        self
    }

    /// Whether to do flood publishing or not. If enabled newly created messages will always be
    /// sent to all peers that are subscribed to the topic and have a good enough score.
    /// The default is true.
    pub fn flood_publish(&mut self, flood_publish: bool) -> &mut Self {
        self.config.flood_publish = flood_publish;
        self
    }

    /// If a GRAFT comes before `graft_flood_threshold` has elapsed since the last PRUNE,
    /// then there is an extra score penalty applied to the peer through P7.
    pub fn graft_flood_threshold(&mut self, graft_flood_threshold: Duration) -> &mut Self {
        self.config.graft_flood_threshold = graft_flood_threshold;
        self
    }

    /// Minimum number of outbound peers in the mesh network before adding more (D_out in the spec).
    /// This value must be smaller or equal than `mesh_n / 2` and smaller than `mesh_n_low`.
    /// The default is 2.
    pub fn mesh_outbound_min(&mut self, mesh_outbound_min: usize) -> &mut Self {
        self.config
            .topic_configuration
            .default_mesh_params
            .mesh_outbound_min = mesh_outbound_min;
        self
    }

    /// Minimum number of outbound peers in the mesh network for a topic before adding more (D_out
    /// in the spec). This value must be smaller or equal than `mesh_n / 2` and smaller than
    /// `mesh_n_low`. The default is 2.
    pub fn mesh_outbound_min_for_topic(
        &mut self,
        mesh_outbound_min: usize,
        topic_hash: TopicHash,
    ) -> &mut Self {
        self.config
            .topic_configuration
            .topic_mesh_params
            .entry(topic_hash)
            .and_modify(|existing_config| {
                existing_config.mesh_outbound_min = mesh_outbound_min;
            })
            .or_insert_with(|| TopicMeshConfig {
                mesh_outbound_min,
                ..TopicMeshConfig::default()
            });
        self
    }

    /// Number of heartbeat ticks that specify the interval in which opportunistic grafting is
    /// applied. Every `opportunistic_graft_ticks` we will attempt to select some high-scoring mesh
    /// peers to replace lower-scoring ones, if the median score of our mesh peers falls below a
    /// threshold (see <https://godoc.org/github.com/libp2p/go-libp2p-pubsub#PeerScoreThresholds>).
    /// The default is 60.
    pub fn opportunistic_graft_ticks(&mut self, opportunistic_graft_ticks: u64) -> &mut Self {
        self.config.opportunistic_graft_ticks = opportunistic_graft_ticks;
        self
    }

    /// Controls how many times we will allow a peer to request the same message id through IWANT
    /// gossip before we start ignoring them. This is designed to prevent peers from spamming us
    /// with requests and wasting our resources.
    pub fn gossip_retransimission(&mut self, gossip_retransimission: u32) -> &mut Self {
        self.config.gossip_retransimission = gossip_retransimission;
        self
    }

    /// The maximum number of new peers to graft to during opportunistic grafting. The default is 2.
    pub fn opportunistic_graft_peers(&mut self, opportunistic_graft_peers: usize) -> &mut Self {
        self.config.opportunistic_graft_peers = opportunistic_graft_peers;
        self
    }

    /// The maximum number of messages we will process in a given RPC. If this is unset, there is
    /// no limit. The default is None.
    pub fn max_messages_per_rpc(&mut self, max: Option<usize>) -> &mut Self {
        self.config.max_messages_per_rpc = max;
        self
    }

    /// The maximum number of messages to include in an IHAVE message.
    /// Also controls the maximum number of IHAVE ids we will accept and request with IWANT from a
    /// peer within a heartbeat, to protect from IHAVE floods. You should adjust this value from the
    /// default if your system is pushing more than 5000 messages in GossipSubHistoryGossip
    /// heartbeats; with the defaults this is 1666 messages/s. The default is 5000.
    pub fn max_ihave_length(&mut self, max_ihave_length: usize) -> &mut Self {
        self.config.max_ihave_length = max_ihave_length;
        self
    }

    /// GossipSubMaxIHaveMessages is the maximum number of IHAVE messages to accept from a peer
    /// within a heartbeat.
    pub fn max_ihave_messages(&mut self, max_ihave_messages: usize) -> &mut Self {
        self.config.max_ihave_messages = max_ihave_messages;
        self
    }

    /// By default, gossipsub will reject messages that are sent to us that has the same message
    /// source as we have specified locally. Enabling this, allows these messages and prevents
    /// penalizing the peer that sent us the message. Default is false.
    pub fn allow_self_origin(&mut self, allow_self_origin: bool) -> &mut Self {
        self.config.allow_self_origin = allow_self_origin;
        self
    }

    /// Time to wait for a message requested through IWANT following an IHAVE advertisement.
    /// If the message is not received within this window, a broken promise is declared and
    /// the router may apply behavioural penalties. The default is 3 seconds.
    pub fn iwant_followup_time(&mut self, iwant_followup_time: Duration) -> &mut Self {
        self.config.iwant_followup_time = iwant_followup_time;
        self
    }

    /// Enable support for flooodsub peers.
    pub fn support_floodsub(&mut self) -> &mut Self {
        if self
            .config
            .protocol
            .protocol_ids
            .contains(&FLOODSUB_PROTOCOL)
        {
            return self;
        }

        self.config.protocol.protocol_ids.push(FLOODSUB_PROTOCOL);
        self
    }

    /// Published message ids time cache duration. The default is 10 seconds.
    pub fn published_message_ids_cache_time(
        &mut self,
        published_message_ids_cache_time: Duration,
    ) -> &mut Self {
        self.config.published_message_ids_cache_time = published_message_ids_cache_time;
        self
    }

    /// The max number of messages a `ConnectionHandler` can buffer. The default is 5000.
    pub fn connection_handler_queue_len(&mut self, len: usize) -> &mut Self {
        self.config.connection_handler_queue_len = len;
        self
    }

    /// The duration a message to be published can wait to be sent before it is abandoned. The
    /// default is 5 seconds.
    pub fn publish_queue_duration(&mut self, duration: Duration) -> &mut Self {
        self.config.connection_handler_publish_duration = duration;
        self
    }

    /// The duration a message to be forwarded can wait to be sent before it is abandoned. The
    /// default is 1s.
    pub fn forward_queue_duration(&mut self, duration: Duration) -> &mut Self {
        self.config.connection_handler_forward_duration = duration;
        self
    }

    /// The message size threshold for which IDONTWANT messages are sent.
    /// Sending IDONTWANT messages for small messages can have a negative effect to the overall
    /// traffic and CPU load. This acts as a lower bound cutoff for the message size to which
    /// IDONTWANT won't be sent to peers. Only works if the peers support Gossipsub1.2
    /// (see <https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/gossipsub-v1.2.md#idontwant-message>)
    /// default is 1kB
    pub fn idontwant_message_size_threshold(&mut self, size: usize) -> &mut Self {
        self.config.idontwant_message_size_threshold = size;
        self
    }

    /// Send IDONTWANT messages after publishing message on gossip. This is an optimisation
    /// to avoid bandwidth consumption by downloading the published message over gossip.
    /// By default it is false.
    pub fn idontwant_on_publish(&mut self, idontwant_on_publish: bool) -> &mut Self {
        self.config.idontwant_on_publish = idontwant_on_publish;
        self
    }

    /// The topic configuration sets mesh parameter sizes for a given topic. Notes on default
    /// below.
    ///
    /// mesh_outbound_min
    /// Minimum number of outbound peers in the mesh network before adding more (D_out in the spec).
    /// This value must be smaller or equal than `mesh_n / 2` and smaller than `mesh_n_low`.
    /// The default is 2.
    ///
    /// mesh_n
    /// Target number of peers for the mesh network for a given topic (D in the spec, default is 6).
    ///
    /// mesh_n_low
    /// Minimum number of peers in mesh network before adding more for a given topic (D_lo in the
    /// spec, default is 4).
    ///
    /// mesh_n_high
    /// Maximum number of peers in mesh network before removing some for a given topic (D_high in
    /// the spec, default is 12).
    pub fn set_topic_config(&mut self, topic: TopicHash, config: TopicMeshConfig) -> &mut Self {
        self.config
            .topic_configuration
            .topic_mesh_params
            .insert(topic, config);
        self
    }

    /// The topic max size sets message sizes for a given topic.
    pub fn set_topic_max_transmit_size(&mut self, topic: TopicHash, max_size: usize) -> &mut Self {
        self.config
            .protocol
            .max_transmit_sizes
            .insert(topic, max_size);
        self
    }

    /// Constructs a [`Config`] from the given configuration and validates the settings.
    pub fn build(&self) -> Result<Config, ConfigBuilderError> {
        // check all constraints on config

        let pre_configured_topics = self.config.protocol.max_transmit_sizes.keys();
        for topic in pre_configured_topics {
            if self.config.protocol.max_transmit_size_for_topic(topic) < 100 {
                return Err(ConfigBuilderError::MaxTransmissionSizeTooSmall);
            }

            let mesh_n = self.config.mesh_n_for_topic(topic);
            let mesh_n_low = self.config.mesh_n_low_for_topic(topic);
            let mesh_n_high = self.config.mesh_n_high_for_topic(topic);
            let mesh_outbound_min = self.config.mesh_outbound_min_for_topic(topic);

            if !(mesh_outbound_min <= mesh_n_low && mesh_n_low <= mesh_n && mesh_n <= mesh_n_high) {
                return Err(ConfigBuilderError::MeshParametersInvalid);
            }

            if mesh_outbound_min * 2 > mesh_n {
                return Err(ConfigBuilderError::MeshOutboundInvalid);
            }
        }

        if self.config.history_length < self.config.history_gossip {
            return Err(ConfigBuilderError::HistoryLengthTooSmall);
        }

        if self.config.unsubscribe_backoff.as_millis() == 0 {
            return Err(ConfigBuilderError::UnsubscribeBackoffIsZero);
        }

        if self.invalid_protocol {
            return Err(ConfigBuilderError::InvalidProtocol);
        }

        Ok(self.config.clone())
    }
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut builder = f.debug_struct("GossipsubConfig");
        let _ = builder.field("protocol", &self.protocol);
        let _ = builder.field("history_length", &self.history_length);
        let _ = builder.field("history_gossip", &self.history_gossip);
        let _ = builder.field(
            "mesh_n",
            &self.topic_configuration.default_mesh_params.mesh_n,
        );
        let _ = builder.field(
            "mesh_n_low",
            &self.topic_configuration.default_mesh_params.mesh_n_low,
        );
        let _ = builder.field(
            "mesh_n_high",
            &self.topic_configuration.default_mesh_params.mesh_n_high,
        );
        let _ = builder.field("retain_scores", &self.retain_scores);
        let _ = builder.field("gossip_lazy", &self.gossip_lazy);
        let _ = builder.field("gossip_factor", &self.gossip_factor);
        let _ = builder.field("heartbeat_initial_delay", &self.heartbeat_initial_delay);
        let _ = builder.field("heartbeat_interval", &self.heartbeat_interval);
        let _ = builder.field("fanout_ttl", &self.fanout_ttl);
        let _ = builder.field("duplicate_cache_time", &self.duplicate_cache_time);
        let _ = builder.field("validate_messages", &self.validate_messages);
        let _ = builder.field("allow_self_origin", &self.allow_self_origin);
        let _ = builder.field("do_px", &self.do_px);
        let _ = builder.field("prune_peers", &self.prune_peers);
        let _ = builder.field("prune_backoff", &self.prune_backoff);
        let _ = builder.field("backoff_slack", &self.backoff_slack);
        let _ = builder.field("flood_publish", &self.flood_publish);
        let _ = builder.field("graft_flood_threshold", &self.graft_flood_threshold);
        let _ = builder.field(
            "mesh_outbound_min",
            &self
                .topic_configuration
                .default_mesh_params
                .mesh_outbound_min,
        );
        let _ = builder.field("opportunistic_graft_ticks", &self.opportunistic_graft_ticks);
        let _ = builder.field("opportunistic_graft_peers", &self.opportunistic_graft_peers);
        let _ = builder.field("max_messages_per_rpc", &self.max_messages_per_rpc);
        let _ = builder.field("max_ihave_length", &self.max_ihave_length);
        let _ = builder.field("max_ihave_messages", &self.max_ihave_messages);
        let _ = builder.field("iwant_followup_time", &self.iwant_followup_time);
        let _ = builder.field(
            "published_message_ids_cache_time",
            &self.published_message_ids_cache_time,
        );
        let _ = builder.field(
            "idontwant_message_size_threshold",
            &self.idontwant_message_size_threshold,
        );
        let _ = builder.field("idontwant_on_publish", &self.idontwant_on_publish);
        builder.finish()
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use libp2p_core::UpgradeInfo;

    use super::*;
    use crate::{topic::IdentityHash, Topic};

    #[test]
    fn create_config_with_message_id_as_plain_function() {
        let config = ConfigBuilder::default()
            .message_id_fn(message_id_plain_function)
            .build()
            .unwrap();

        let result = config.message_id(&get_gossipsub_message());

        assert_eq!(result, get_expected_message_id());
    }

    #[test]
    fn create_config_with_message_id_as_closure() {
        let config = ConfigBuilder::default()
            .message_id_fn(|message: &Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                let mut v = s.finish().to_string();
                v.push('e');
                MessageId::from(v)
            })
            .build()
            .unwrap();

        let result = config.message_id(&get_gossipsub_message());

        assert_eq!(result, get_expected_message_id());
    }

    #[test]
    fn create_config_with_message_id_as_closure_with_variable_capture() {
        let captured: char = 'e';

        let config = ConfigBuilder::default()
            .message_id_fn(move |message: &Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                let mut v = s.finish().to_string();
                v.push(captured);
                MessageId::from(v)
            })
            .build()
            .unwrap();

        let result = config.message_id(&get_gossipsub_message());

        assert_eq!(result, get_expected_message_id());
    }

    #[test]
    fn create_config_with_protocol_id_prefix() {
        let protocol_config = ConfigBuilder::default()
            .protocol_id_prefix("/purple")
            .build()
            .unwrap()
            .protocol_config();

        let protocol_ids = protocol_config.protocol_info();

        assert_eq!(protocol_ids.len(), 2);

        assert_eq!(
            protocol_ids[0].protocol,
            StreamProtocol::new("/purple/1.1.0")
        );
        assert_eq!(protocol_ids[0].kind, PeerKind::Gossipsubv1_1);

        assert_eq!(
            protocol_ids[1].protocol,
            StreamProtocol::new("/purple/1.0.0")
        );
        assert_eq!(protocol_ids[1].kind, PeerKind::Gossipsub);
    }

    #[test]
    fn create_config_with_custom_protocol_id() {
        let protocol_config = ConfigBuilder::default()
            .protocol_id("/purple", Version::V1_0)
            .build()
            .unwrap()
            .protocol_config();

        let protocol_ids = protocol_config.protocol_info();

        assert_eq!(protocol_ids.len(), 1);

        assert_eq!(protocol_ids[0].protocol, "/purple");
        assert_eq!(protocol_ids[0].kind, PeerKind::Gossipsub);
    }

    fn get_gossipsub_message() -> Message {
        Message {
            source: None,
            data: vec![12, 34, 56],
            sequence_number: None,
            topic: Topic::<IdentityHash>::new("test").hash(),
        }
    }

    fn get_expected_message_id() -> MessageId {
        MessageId::from([
            49, 55, 56, 51, 56, 52, 49, 51, 52, 51, 52, 55, 51, 51, 53, 52, 54, 54, 52, 49, 101,
        ])
    }

    fn message_id_plain_function(message: &Message) -> MessageId {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        let mut v = s.finish().to_string();
        v.push('e');
        MessageId::from(v)
    }
}
