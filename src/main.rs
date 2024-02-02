use futures::stream::StreamExt;
use libp2p::Swarm;
use rand::Rng;
use libp2p::{gossipsub, mdns, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use tokio::{io, io::AsyncBufReadExt, select};
use pretty_env_logger;
// We create a custom network behaviour that combines Gossipsub and Mdns.
#[derive(NetworkBehaviour)]
struct MyBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let mut swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        // .with_websocket(
        //     noise::Config::new,
        //     yamux::Config::default,
        // ).await?
        .with_quic()
        .with_behaviour(|key| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let message_id_fn = |message: &gossipsub::Message| {
                let mut s = DefaultHasher::new();
                message.data.hash(&mut s);
                gossipsub::MessageId::from(s.finish().to_string())
            };

            // Set a custom gossipsub configuration
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
                .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
                .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
                .build()
                .map_err(|msg| io::Error::new(io::ErrorKind::Other, msg))?; // Temporary hack because `build` does not return a proper `std::error::Error`.

            // build a gossipsub network behaviour
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            Ok(MyBehaviour { gossipsub, mdns })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    
    let mut rng = rand::thread_rng();
    let t = format!("topic-{}", rng.gen::<u32>());
    println!("Topic: {}", t);
    
    // topic for uni & multi cast purpose
    let topic = gossipsub::IdentTopic::new(&t);
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    
    // topic for broadcast purpose
    let bcast_topic = &gossipsub::IdentTopic::new("topic-broadcast");
    swarm.behaviour_mut().gossipsub.subscribe(&bcast_topic)?;
    

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    // swarm.listen_on("/ip4/0.0.0.0/tcp/0/ws".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    println!("Enter messages via STDIN and they will be sent to connected peers using Gossipsub");

    // Kick it off
    loop {
        select! {
            Ok(Some(line)) = stdin.next_line() => {
                let _ = handle_ip(line.to_string(), &bcast_topic, &mut swarm);
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discovered a new peer: {peer_id}");
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                    for (peer_id, _multiaddr) in list {
                        println!("mDNS discover peer has expired: {peer_id}");
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                    }
                },
                SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message_id: id,
                    message,
                })) => println!(
                        "Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    ),
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Local node is listening on {address}");
                }
                _ => {}
            }
        }
    }
}

fn handle_ip(line: String, bcast_topic: &gossipsub::IdentTopic, swarm: &mut Swarm<MyBehaviour>) -> Result<(), Box<dyn std::error::Error>> {
    if line.contains("t:") {
        // uni & multi cast
        // t:<>:<>
        let s: Vec<&str> = line.split(':').collect();
        
        if s[1] == "sub" { // subscribe to given topic
            // t:sub:<topic>
            let tmp_topic = gossipsub::IdentTopic::new(s[2]);
            if let Err(_) = swarm.behaviour_mut().gossipsub.subscribe(&tmp_topic) {
                println!("failed to subscribe to the topic: {}", s[2]);
            } else {
                println!("subscription success!");
            }
        } else {
            // t:<topic>:<msg>
            let tmp_topic = gossipsub::IdentTopic::new(s[1]);
            if let Err(e) = swarm
                .behaviour_mut().gossipsub
                .publish(tmp_topic.clone(), s[2].as_bytes()) {
                println!("Publish error: {e:?}");
            }
        }
    } else {
        // broadcast
        if let Err(e) = swarm
            .behaviour_mut().gossipsub
            .publish(bcast_topic.clone(), line.as_bytes()) {
            println!("Publish error: {e:?}");
        }
    }
    Ok(())
}