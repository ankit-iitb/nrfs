// Copyright © 2019-2020 VMware, Inc. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Allows to query information about the CPU topology.
#![allow(unused)]

use alloc::fmt::{Debug, Formatter, Result};
use alloc::vec::Vec;
use hwloc2::*;

pub type Node = u64;
pub type Socket = u64;
pub type Core = u64;
pub type Cpu = u64;
pub type L1 = u64;
pub type L2 = u64;
pub type L3 = u64;

/// NUMA Node information.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub struct NodeInfo {
    /// Node index
    pub node: Node,
    /// Memory in bytes
    pub memory: u64,
}

/// Information about a CPU in the system.
#[derive(Eq, PartialEq, Clone, Copy)]
pub struct CpuInfo {
    pub node: Option<NodeInfo>,
    pub socket: Socket,
    pub core: Core,
    pub cpu: Cpu,
    pub l1: L1,
    pub l2: L2,
    pub l3: L3,
}

impl Debug for CpuInfo {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "CpuInfo {{ core/l1/l2: {}/{}/{}, cpu: {}, socket/l3/node: {}/{}/{:?} }}",
            self.core, self.l1, self.l2, self.cpu, self.socket, self.l3, self.node
        )
    }
}

#[derive(Debug)]
pub struct MachineTopology {
    data: Vec<CpuInfo>,
}

impl MachineTopology {
    pub fn new() -> MachineTopology {
        let mut data: Vec<CpuInfo> = Default::default();

        let topo = Topology::new().expect("Can't retrieve Topology");
        let cpus = topo
            .objects_with_type(&ObjectType::PU)
            .expect("Can't find CPUs");

        for cpu in cpus {
            let mut parent = cpu.parent();

            // Find the parent core of the CPU
            while parent.is_some() && parent.unwrap().object_type() != ObjectType::Core {
                parent = parent.unwrap().parent();
            }
            let core = parent.expect("PU has no Core?");

            // Find the parent L1 cache of the CPU
            while parent.is_some()
                && (parent.unwrap().object_type() != ObjectType::L1Cache
                    || parent.unwrap().cache_attributes().unwrap().depth() < 1)
            {
                parent = parent.unwrap().parent();
            }
            let l1 = parent.expect("Core doesn't have a L1 cache?");

            // Find the parent L2 cache of the CPU
            while parent.is_some()
                && (parent.unwrap().object_type() != ObjectType::L2Cache
                    || parent.unwrap().cache_attributes().unwrap().depth() < 2)
            {
                parent = parent.unwrap().parent();
            }
            let l2 = parent.expect("Core doesn't have a L2 cache?");

            // Find the parent socket/L3 cache of the CPU
            while parent.is_some()
                && (parent.unwrap().object_type() != ObjectType::L3Cache
                    || parent.unwrap().cache_attributes().unwrap().depth() < 3)
            {
                parent = parent.unwrap().parent();
            }
            let socket = parent.expect("Core doesn't have a L3 cache (socket)?");

            // Find the parent NUMA node of the CPU
            while parent.is_some() && parent.unwrap().object_type() != ObjectType::NUMANode {
                parent = parent.unwrap().parent();
            }
            let numa_node = parent.map(|n| NodeInfo {
                node: n.os_index() as Node,
                memory: n.total_memory(),
            });

            let cpu_info = CpuInfo {
                node: numa_node,
                socket: socket.logical_index() as Socket,
                core: core.logical_index() as Core,
                cpu: cpu.os_index() as Cpu,
                l1: l1.logical_index() as L1,
                l2: l2.logical_index() as L2,
                l3: socket.logical_index() as L3,
            };

            data.push(cpu_info);
        }

        MachineTopology { data }
    }

    /// Return how many processing units that the system has
    pub fn cores(&self) -> usize {
        self.data.len()
    }

    pub fn sockets(&self) -> Vec<Socket> {
        let mut sockets: Vec<Cpu> = self.data.iter().map(|t| t.socket).collect();
        sockets.sort();
        sockets.dedup();
        sockets
    }

    pub fn cpus_on_socket(&self, socket: Socket) -> Vec<&CpuInfo> {
        self.data.iter().filter(|t| t.socket == socket).collect()
    }
}
