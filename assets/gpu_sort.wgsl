struct SpatialIndex {
    original_index: u32,
    hash: u32,
    key: u32,
};

@group(0) @binding(0) var<storage, read_write> spatial_indices: array<SpatialIndex>;
@group(0) @binding(1) var<uniform> num_entries: u32;
@group(0) @binding(2) var<uniform> group_width: u32;
@group(0) @binding(3) var<uniform> group_height: u32;
@group(0) @binding(4) var<uniform> step_index: u32;
@group(0) @binding(5) var<storage, read_write> spatial_offsets: array<u32>;

// Sort the given spatial_indices by their keys (smallest to largest)
// This is done using bitonic merge sort, and takes multiple iterations
@compute @workgroup_size(128, 1, 1)
fn sort_spatial_indices(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;

    let h_index = i & (group_width - 1);
    let index_left = h_index + (group_height + 1) * (i / group_width);
    var right_step_size: u32 = u32(0);
    if step_index == 0 {
        right_step_size = group_height - 2 * h_index;
    } else {
        right_step_size = (group_height + 1) / 2;
    }
    let index_right = index_left + right_step_size;

    // Exit if out of bounds (for non-power of 2 input sizes)
    if index_right >= num_entries {
        return;
    }

    let value_left = spatial_indices[index_left].key;
    let value_right = spatial_indices[index_right].key;

    // Swap spatial_indices if value is descending
    if value_left > value_right {
        let temp = spatial_indices[index_left];
        spatial_indices[index_left] = spatial_indices[index_right];
        spatial_indices[index_right] = temp;
    }
}


// Calculate offsets into the sorted Entries buffer (used for spatial hashing).
// For example, given an Entries buffer sorted by key like so: {2, 2, 2, 3, 6, 6, 9, 9, 9, 9}
// The resulting Offsets calculated here should be:            {-, -, 0, 3, -, -, 4, -, -, 6}
// (where '-' represents elements that won't be read/written)
// 
// Usage example:
// Say we have a particular particle P, and we want to know which particles are in the same grid cell as it.
// First we would calculate the Key of P based on its position. Let's say in this example that Key = 9.
// Next we can look up Offsets[Key] to get: Offsets[9] = 6
// This tells us that SortedEntries[6] is the first particle that's in the same cell as P.
// We can then loop until we reach a particle with a different cell key in order to iterate over all the particles in the cell.
// 
// NOTE: offsets buffer must filled with values equal to (or greater than) its length to ensure that this works correctly
@compute @workgroup_size(128, 1, 1)
fn calculate_offsets(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= num_entries {
        return;
    }

    let i = id.x;
    let null_val = num_entries;

    let key = spatial_indices[i].key;
    var key_prev: u32 = u32(0);
    if i == 0 {
        key_prev = null_val;
    } else {
        key_prev = spatial_indices[i - 1].key;
    }

    if key != key_prev {
        spatial_offsets[key] = i;
    }
}