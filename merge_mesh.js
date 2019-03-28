const fs = require('fs');

let mesh1_path = process.argv[2];
let mesh2_path = process.argv[3];

let mesh1 = JSON.parse(fs.readFileSync(mesh1_path));
let mesh2 = JSON.parse(fs.readFileSync(mesh2_path));

let final_mesh = JSON.parse(fs.readFileSync(mesh1_path));

mesh2.buffers.forEach(v => {
    final_mesh.buffers.push(v);
});

mesh2.bufferViews.forEach(v => {
    v.buffer += mesh1.buffers.length;
    final_mesh.bufferViews.push(v);
});

mesh2.accessors.forEach(v => {
    v.bufferView += mesh1.bufferViews.length;
    final_mesh.accessors.push(v);
});

mesh2.images.forEach(v => {
    final_mesh.images.push(v);
});

mesh2.samplers.forEach(v => {
    final_mesh.samplers.push(v);
});

mesh2.textures.forEach(v => {
    v.source += mesh1.images.length;
    v.sampler += mesh1.samplers.length;
    final_mesh.textures.push(v);
});

mesh2.materials.forEach(v => {
    v.pbrMetallicRoughness.baseColorTexture.index += mesh1.textures.length;
    v.normalTexture.index += mesh1.textures.length;
    final_mesh.materials.push(v);
});

let node_names = {};
mesh1.nodes.forEach((v, idx) => {
    if (!v.hasOwnProperty('name')) return;
    node_names[v.name] = idx;
});
let added_nodes = [];
let existing_nodes = [];
mesh2.nodes.forEach(v => {
    if (!v.hasOwnProperty('name')) return;
    if (node_names.hasOwnProperty(v.name)) {
        existing_nodes.push(v);
        return;
    }
    let new_index = final_mesh.nodes.length;
    node_names[v.name] = new_index;
    final_mesh.nodes.push(v);
    added_nodes.push(v);
});

added_nodes.forEach(v => {
    if (!v.hasOwnProperty('children')) return;
    v.children = v.children.map(e => node_names[mesh2.nodes[e].name]);
});

existing_nodes.forEach(v => {
    if (!v.hasOwnProperty('name')) return;
    if (!v.hasOwnProperty('children')) return;
    let real_node = final_mesh.nodes[node_names[v.name]];
    if (!real_node.hasOwnProperty('children')) {
        real_node.children = [];
    }

    for (let i = 0; i < v.children.length; i++) {
        let child = v.children[i];
        let child_node = node_names[mesh2.nodes[child].name];
        if (!real_node.children.includes(child_node)) {
            real_node.children.push(child_node);
        }
    }
});

mesh2.meshes.forEach(v => {
    v.primitives = v.primitives.map(primitive => {
        primitive.indices += mesh1.accessors.length;
        primitive.material += mesh1.materials.length;
        for (attribute in primitive.attributes) {
            primitive.attributes[attribute] += mesh1.accessors.length;
        }

        return primitive;
    });
    final_mesh.meshes.push(v);
});

mesh2.skins.forEach(v => {
    v.joints = v.joints.map(e => node_names[mesh2.nodes[e].name]);
    v.inverseBindMatrices += mesh1.accessors.length;
    final_mesh.skins.push(v);
});

// hard coded for now
final_mesh.scenes[0].nodes.push(final_mesh.nodes.length);
final_mesh.nodes.push({
    mesh: mesh1.meshes.length,
    skin: mesh1.skins.length,
});

fs.writeFileSync('./merged.gltf', JSON.stringify(final_mesh));
