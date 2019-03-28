const fs = require('fs');

let mesh_path = process.argv[2];
let anim_path = process.argv[3];

let mesh = JSON.parse(fs.readFileSync(mesh_path));
let anim = JSON.parse(fs.readFileSync(anim_path));

let final_mesh = JSON.parse(fs.readFileSync(mesh_path));

anim.buffers.forEach(v => {
    final_mesh.buffers.push(v);
});

anim.bufferViews.forEach(v => {
    v.buffer += mesh.buffers.length;
    final_mesh.bufferViews.push(v);
});

anim.accessors.forEach(v => {
    v.bufferView += mesh.bufferViews.length;
    final_mesh.accessors.push(v);
});

anim.animations.forEach(v => {
    v.samplers = v.samplers.map(e => ({
        input: e.input + mesh.accessors.length,
        output: e.output + mesh.accessors.length,
        interpolation: e.interpolation,
    }));

    v.channels = v.channels.map(e => {
        let targetNode = false;
        mesh.nodes.forEach((node, idx) => {
            if (!node.hasOwnProperty('name')) return;
            if (node.name.toLowerCase() == e.target.extras.toLowerCase()) {
                targetNode = idx;
            }
        });
        if (targetNode !== false) {
            e.target.node = targetNode;
            return e;
        }
        return false;
    }).filter(e => e !== false);

    final_mesh.animations.push(v);
});

fs.writeFileSync('./merged.gltf', JSON.stringify(final_mesh));
