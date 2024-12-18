use crate::symmetric::tweak_hash::TweakableHash;

/// Hash-Tree based on a tweakable hash function
/// We consider hash trees in which each leaf is first
/// hashed individually.
pub struct HashTree<TH: TweakableHash> {
    /// Layers of the hash tree, starting with the
    /// bottom layer. The leafs are not included: the
    /// bottom layer is the list of hashes of all leafs
    layers: Vec<Vec<TH::Domain>>,
}

/// Function to compute a hash-tree given the leafs as input.
/// The number of leafs must be a power of two.
pub fn build_tree<TH: TweakableHash>(
    parameter: &TH::Parameter,
    leafs: &[&[TH::Domain]],
) -> HashTree<TH> {
    // check that number of leafs is a power of two
    assert!(
        leafs.len().is_power_of_two(),
        "Hash-Tree build_tree: Number of leafs should be power of two"
    );

    let mut layer_size = leafs.len();
    let mut layers: Vec<Vec<TH::Domain>> = Vec::new();

    // the bottom layer contains the individual hashes of all leafs
    layers.push(Vec::new());
    for i in 0..layer_size {
        let tweak = TH::tree_tweak(0, i as u64);
        let hash = TH::apply(parameter, &tweak, leafs[i]);
        layers[0].push(hash);
    }

    // now, we build each layer by hashing pairs in the previous layer
    let mut level = 1;
    while layer_size >= 2 {
        // start a new layer
        layers.push(Vec::new());
        // this new layer will have half the size
        layer_size = layer_size / 2;
        for i in 0..layer_size {
            let left_idx = 2 * i;
            let right_idx = 2 * i + 1;
            let tweak = TH::tree_tweak(level, i as u64);
            let children = &layers[(level - 1) as usize][left_idx..=right_idx];
            let parent = TH::apply(parameter, &tweak, children);
            layers[level as usize].push(parent);
        }
        level += 1;
    }

    HashTree { layers }
}

/// Function to get a root from a tree. The tree must have at least one layer.
/// A root is just an output of the tweakable hash.
pub fn hash_tree_root<TH: TweakableHash>(tree: &HashTree<TH>) -> TH::Domain {
    assert!(
        !tree.layers.is_empty(),
        "Hash-Tree hash tree root: Need at least one layer"
    );
    tree.layers.last().unwrap()[0].clone()
}

/// Opening in a hash-tree: a co-path, without the leaf
pub struct HashTreeOpening<TH: TweakableHash> {
    /// The co-path needed to verify
    /// If the tree has depth h, i.e, 2^h leafs
    /// the co-path should have size D
    co_path: Vec<TH::Domain>,
}

/// Function to compute the Merkle authentication path
/// from a tree and the position of the leaf. It is assumed
/// that the tree is well-formed, i.e., each layer is half
/// the size of the previous layer, and the final layer has
/// size 1.
pub fn hash_tree_path<TH: TweakableHash>(
    tree: &HashTree<TH>,
    position: u64,
) -> HashTreeOpening<TH> {
    assert!(
        !tree.layers.is_empty(),
        "Hash-Tree hash tree path: Need at least one layer"
    );
    assert!(
        position < tree.layers[0].len() as u64,
        "Hash-Tree hash tree path: Invalid position"
    );

    // in our co-path, we will have one node per layer
    // except the final layer (which is just the root)
    let depth = tree.layers.len() - 1;
    let mut co_path: Vec<TH::Domain> = Vec::with_capacity(depth);
    let mut current_position = position;
    for l in 0..depth {
        // position of the sibling that we want to include
        let sibling_position = current_position ^ 0x01;
        // add to the co-path
        let sibling = tree.layers[l][sibling_position as usize].clone();
        co_path.push(sibling);
        // new position in next layer
        current_position = current_position >> 1;
    }

    HashTreeOpening { co_path }
}

/// Function to verify an Merkle authentication path
/// with respect to a root, a position, and a leaf.
pub fn hash_tree_verify<TH: TweakableHash>(
    parameter: &TH::Parameter,
    root: &TH::Domain,
    position: u64,
    leaf: &[TH::Domain],
    opening: &HashTreeOpening<TH>,
) -> bool {
    // given the length of the path, we know how
    // large the tree was. So we can check if the
    // position makes sense.
    let depth = opening.co_path.len();
    let num_leafs: u64 = 1 << depth;
    assert!(
        position < num_leafs,
        "Hash-Tree hash tree verify: Position and Path Length not compatible"
    );

    // first hash the leaf to get the node in the bottom layer
    let tweak = TH::tree_tweak(0, position);
    let mut current_node = TH::apply(parameter, &tweak, leaf);

    // now reconstruct the root using the co-path
    let mut current_position = position;
    for l in 0..depth {
        // Need to distinguish two cases, depending on
        // if current is a left child or a right child
        let children = if current_position % 2 == 0 {
            // left child, so co-path contains the right sibling
            [current_node, opening.co_path[l]]
        } else {
            // right child, so co-path contains the left sibling
            [opening.co_path[l], current_node]
        };

        // determine new position, which is position of the parent
        current_position = current_position >> 1;

        // now hash to get the parent
        let tweak = TH::tree_tweak(l as u64, current_position);
        current_node = TH::apply(parameter, &tweak, &children);
    }

    // Finally, check that recomputed root matches given root
    current_node == *root
}

// TODO. Tests.
