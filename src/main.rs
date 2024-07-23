use libp2p::{identity, mplex, NetworkBehaviour, PeerId, Swarm, Transport};
use libp2p::core::upgrade;
use libp2p::floodsub::{Floodsub, Topic};
use libp2p::futures::StreamExt;
use libp2p::mdns::Mdns;
use libp2p::noise::{Keypair, NoiseConfig, X25519Spec};
use libp2p::swarm::SwarmBuilder;
use libp2p::tcp::TokioTcpConfig;
use log::info;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;

const STORAGE_FILE_PATH: &str = "./recipes.json";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

static KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("recipes"));

type Recipes = Vec<Recipe>;

#[derive(Debug, Serialize, Deserialize)]
struct Recipe {
    id: usize,
    name: String,
    ingredients: String,
    instructions: String,
    public: bool,
}

#[derive(Debug, Serialize, Deserialize)]
enum ListMode {
    ALL,
    One(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ListRequest {
    mode: ListMode,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListResponse {
    mode: ListMode,
    data: Recipes,
    receiver: String,
}

enum EventType {
    Response(ListResponse),
    Input(String),
}

#[derive(NetworkBehaviour)]
struct RecipeBehaviour {
    floodsub: Floodsub,
    mdns: Mdns,
    #[behaviour(ignore)]
    response_sender: mpsc::UnboundedSender<ListResponse>,
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    info!("Peer Id: {}",PEER_ID.clone());
    let (response_sender, mut response_rcv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&KEYS)
        .expect("can create auth keys");

    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    let mut behaviour = RecipeBehaviour {
        floodsub: Floodsub::new(PEER_ID.clone()),
        mdns: Mdns::new(Default::default())
            .await.expect("can create mdns"),
        response_sender,
    };

    behaviour.floodsub.subscribe(TOPIC.clone());

    let mut swarm = SwarmBuilder::new(transp, behaviour, PEER_ID.clone())
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    Swarm::listen_on(
        &mut swarm,
        "/ip4/0.0.0.0/tcp/0"
            .parse()
            .expect("can get a local socket"),
    )
        .expect("swarm can be started");

    loop {
        let evt={
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                event = swarm.select_next_some() => {
                    info!("Unhandled Swarm Event: {:?}", event);
                    None
                },
                response = response_rcv.recv()=>Some(EventType::Response(response.expect("response exists"))),
            }
        };
        if let Some(event)=evt{
            match event {
                EventType::Response(resp) => {
                    //todo
                }
                EventType::Input(line) => {
                    //todo
                }
            }
        }
    }
}
