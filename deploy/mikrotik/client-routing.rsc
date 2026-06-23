# mjolnir-mesh — mesh client routing for RouterOS.
#
# Apply on EVERY router AFTER container-net.rsc has run. Idempotent
# (safe to re-run). Adds the single static route that sends all mesh
# client traffic (10.42.0.0/16) to the meshd container, plus two
# firewall forward-accept rules so transit packets aren't dropped by a
# default forward-drop policy.
#
# Upload this file, then:  /import file-name=client-routing.rsc
#
# Route installed:  10.42.0.0/16 via 172.20.0.2 (the container gateway)
# Firewall rules:   forward accept  src-address=10.42.0.0/16  (mesh → local)
#                   forward accept  dst-address=10.42.0.0/16  (local → mesh)

:local meshNet "10.42.0.0/16"
:local containerGw "172.20.0.2"

# Static route — send all mesh client traffic to the container.
# babeld inside the container knows which per-/24 goes to which TUN peer.
:if ([:len [/ip/route/find where dst-address=$meshNet gateway=$containerGw]] = 0) do={
    /ip/route/add dst-address=$meshNet gateway=$containerGw \
        comment="mjolnir mesh clients"
}

# Firewall — accept transit traffic to/from the mesh client supernet.
# Both directions are needed on a router with a default forward-drop:
#   src rule: packets leaving the mesh toward this router's LAN
#   dst rule: packets from this router's LAN toward the mesh
# On a blank firewall these rules are harmless no-ops.
# Placed at the top (place-before=0) so a later drop can't pre-empt them.

:if ([:len [/ip/firewall/filter/find where comment="mjolnir mesh transit src"]] = 0) do={
    :if ([:len [/ip/firewall/filter/find]] > 0) do={
        /ip/firewall/filter/add chain=forward action=accept src-address=$meshNet \
            comment="mjolnir mesh transit src" place-before=0
    } else={
        /ip/firewall/filter/add chain=forward action=accept src-address=$meshNet \
            comment="mjolnir mesh transit src"
    }
}

:if ([:len [/ip/firewall/filter/find where comment="mjolnir mesh transit dst"]] = 0) do={
    :if ([:len [/ip/firewall/filter/find]] > 0) do={
        /ip/firewall/filter/add chain=forward action=accept dst-address=$meshNet \
            comment="mjolnir mesh transit dst" place-before=0
    } else={
        /ip/firewall/filter/add chain=forward action=accept dst-address=$meshNet \
            comment="mjolnir mesh transit dst"
    }
}

:put "mjolnir client-routing: done. 10.42.0.0/16 -> 172.20.0.2 route + 2x forward-accept in place."
