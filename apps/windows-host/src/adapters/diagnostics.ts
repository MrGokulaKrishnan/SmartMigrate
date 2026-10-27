// TODO: wire to Rust command run_diagnostics

export interface DiagnosticCheckResult {
  id: string;
  name: string;
  detail: string;
  status: "pass" | "warn" | "fail";
  message: string;
}

export async function runDiagnosticChecks(lanIp: string = "192.168.31.33"): Promise<DiagnosticCheckResult[]> {
  // Simulates a sequence of real probes for TCP 7890, UDP 7889, firewall, network profile, advertised IP, and encryption
  return [
    {
      id: "tcp_7890",
      name: "Listening on TCP 7890",
      detail: "Pairing and streaming port",
      status: "pass",
      message: "Listening on 0.0.0.0:7890",
    },
    {
      id: "udp_7889",
      name: "Discovery on UDP 7889",
      detail: "Lets phones find this PC",
      status: "pass",
      message: "Discovery answering on 0.0.0.0:7889",
    },
    {
      id: "firewall",
      name: "Firewall rules present",
      detail: "Inbound rules for both ports on private networks",
      status: "pass",
      message: "Rules present for both ports",
    },
    {
      id: "network_profile",
      name: "Network profile",
      detail: "Private networks only accept discovery",
      status: "pass",
      message: "Private network. Discovery accepted.",
    },
    {
      id: "advertised_ip",
      name: "Advertised address",
      detail: "The IP this PC gives phones",
      status: "pass",
      message: `${lanIp || "192.168.31.33"} is the default-route address`,
    },
    {
      id: "transport_encryption",
      name: "Transport encryption",
      detail: "Not enabled yet. Traffic on this network is readable.",
      status: "warn",
      message: "Not enabled yet. Traffic is readable on this network.",
    },
  ];
}
