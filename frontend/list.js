import { w2grid, w2popup } from 'https://rawgit.com/vitmalina/w2ui/master/dist/w2ui.es6.min.js';

const NODE_TYPE_OPTIONS = [
    { id: 1, text: 'Random' },
    { id: 2, text: 'RoundRobin' },
    { id: 3, text: 'Reverse' },
    { id: 4, text: 'Derailing' },
    { id: 5, text: 'Configurable' }
];

let grid_node = new w2grid({
    name: 'nodelist',
    header: 'Node List',
    box: '#grid-node',
    show: {
        toolbar: true,
        footer: true,
        lineNumbers: true,
        toolbarSave: false,
        header: true
    },
    columns: [
        { field: 'recid', text: 'NodeID', size: '150px', sortable: true, resizable: true },
        {
            field: 'PositionX', text: 'PositionX', size: '180px', sortable: true, resizable: true, render: 'int',
            editable: { type: 'int', min: 0, max: 32756 }
        },
        {
            field: 'PositionY', text: 'PositionY', size: '180px', sortable: true, resizable: true, render: 'int',
            editable: { type: 'int', min: 0, max: 32756 }
        },
        {
            field: 'nodetype', text: 'Type', size: '100px', sortable: true, resizable: true,
            editable: { type: 'list', items: NODE_TYPE_OPTIONS, showAll: true, openOnFocus: true, align: 'left' },
            render(record, extra) {
                return extra.value?.text || record.nodetype;
            }
        },
    ],
    toolbar: {
        items: [
            { type: 'button', id: 'pushChanges', text: 'Push Changes', icon: 'w2ui-icon-check' },
            { id: 'add', type: 'button', text: 'Add Record', icon: 'w2ui-icon-plus' },
            { type: 'break' },
            { type: 'button', id: 'showChanges', text: 'Show Changes' },

        ],
        onClick(event) {
            if (event.target == 'add') {
                ctrl_socket.send("node_new\n0;0 random");
                // window.setTimeout(() => {
                //     let last_rec = grid_node.records[grid_node.records.length - 1];
                //     this.owner.scrollIntoView(last_rec.recid);
                //     this.owner.editField(last_rec.recid, 1);
                // }, 100);
            }
            if (event.target == 'showChanges') {
                showChanged()
            }
            if (event.target == 'pushChanges') {
                //get change
                let change = grid_node.getChanges();
                grid_node.save();
                change.forEach(e => {
                    let record = grid_node.get(e.recid);
                    //you can access node type  by record.nodetype
                    ctrl_socket.send("node_move\n" + e.recid + " " + record.PositionX + ";" + record.PositionY);
                });

            }
        }
    },
    records: [
    ]
})

window.showChanged = function () {
    w2popup.open({
        title: 'Records Changes',
        with: 600,
        height: 550,
        body: `<pre>${JSON.stringify(grid_node.getChanges(), null, 4)}</pre>`,
        actions: { Ok: w2popup.close }
    })
}

let grid_track = new w2grid({
    name: 'tracklist',
    header: 'Track List',
    box: '#grid-track',
    show: {
        toolbar: true,
        footer: true,
        lineNumbers: true,
        toolbarSave: false,
        header: true
    },
    columns: [
        { field: 'recid', text: 'TrackID', size: '50px', sortable: true, resizable: true },
        {
            field: 'start', text: 'Start', size: '180px', sortable: true, resizable: true,
            editable: false
        },
        {
            field: 'end', text: 'End', size: '180px', sortable: true, resizable: true,
            editable: false
        },
        {
            field: 'color', text: 'Color', size: '180px', sortable: true, resizable: true,
            editable: true,
            render(record, extra) {
                //return color of the value
                //req styling
                return '<span style="background-color: ' + (extra.value?.text || record.color) + '">' + (extra.value?.text || record.color) + '</span>';
            }
        },
        {
            field: 'action', text: 'Action', size: '200px', sortable: false, resizable: true,
            editable: false,
            render(record, extra) {
                return "<button  class='bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded-full' onclick=\"sendNewTrainRequest(" + record.recid + ");\">New Train</button>";
            }
        },
    ],
    toolbar: {
        items: [
            //those commented are preserved for color changing
            // { type: 'button', id: 'pushChanges', text: 'Push Changes', icon: 'w2ui-icon-check' },
            { id: 'add', type: 'button', text: 'Add Record', icon: 'w2ui-icon-plus' },
            // { type: 'break' },
            // { type: 'button', id: 'showChanges', text: 'Show Changes' },

        ],
        onClick(event) {
            if (event.target == 'add') {
                let recid = grid_track.records[grid_track.records.length - 1].recid + 1;
                //not this easy have to open a pop up windows and ask for input, also push change at the same time
                w2popup.open({
                    width: 580,
                    height: 400,
                    title: 'Adding new track',
                    focus: 0,
                    body: `
    <div class="w2ui-centered" style="line-height: 1.8">
<div>
<span tabindex="0">Enter two valid node id to draw a new track.</span>
<br><br>
<div class="w2ui-field">
<label for="startnodeid">Start NodeID:</label>
<div>
   <input name="startnodeid" class="w2ui-input" id="inputstartnodeid" style="margin-bottom: 5px">
</div>
</div>
<div class="w2ui-field">
<label for="endnodeid">End NodeID:</label>
<div>
   <input name="endnodeid" class="w2ui-input" id="inputendnodeid" style="margin-bottom: 5px">
</div>
</div>
<div class="w2ui-field">
  <label for="color">Color:</label>
  <div>
     <input name="color" class="w2ui-input" id="inputcolor" style="margin-bottom: 5px">
  </div>
</div>
<br>
</div>
</div>`,
                    actions: {
                        Ok() {
                            console.log('Ok button is clicked');
                            ctrl_socket.send(
                                'track_new\n' + document.getElementById('inputstartnodeid').value + ' '
                                + document.getElementById('inputendnodeid').value
                                + ' #' + ((1 << 24) * Math.random() | 0).toString(16).padStart(6, '0')
                            );//even user can set color value but still assign random value and ignore user input
                            w2popup.close()
                            let recid = grid_track.records[grid_track.records.length - 1].recid + 1
                            grid_track.scrollIntoView(recid);
                        },
                        Cancel() {
                            w2popup.close()
                        }
                    },
                    // onKeydown(event) {
                    //     console.log('keydown', event)
                    // },
                    // onMove(event) {
                    //     console.log('popup moved', event)
                    // }
                });
            }
            if (event.target == 'showChanges') {
                showChanged()
            }
            if (event.target == 'pushChanges') {
                let change = grid_track.getChanges();
                grid_track.save();
                change.forEach(e => {
                    record = grid_track.get(e.recid);
                    //this is for changing color preserved for future

                });

            }
        }
    },
    records: [
    ]
});

let grid_train = new w2grid({
    name: 'trainlist',
    header: 'Train List',
    box: '#grid-train',
    show: {
        toolbar: false,
        footer: true,
        lineNumbers: true,
        toolbarSave: false,
        header: true
    },
    columns: [
        { field: 'recid', text: 'TrainID', size: '70px', sortable: true, resizable: true, clipboardCopy: true },
        {
            field: 'track', text: 'Track', size: '50px', sortable: true, resizable: true, render: 'int',
            editable: false
        },
        {
            field: 'direction', text: 'Direction', size: '70px', sortable: true, resizable: true,
            editable: false, comment: 'w2ui: { style: "background-color: #C2F5B4" }',
            //replace rev function
            // editable: { type: 'list', items: typeoption, showAll: true, openOnFocus: true, align: 'left' },
            render(record, extra) {//function rev should do
                return record.direction;
            }
        },
        {
            field: 'image', text: 'Image', size: '300px', sortable: true, resizable: true,
            editable: false,
            render(record, extra) {//maybe we can render the image here
                return "<span><img src=" + record.image + " alt='train image' style='height:1em;display:inline;'>" + record.image + "</span>";
            }
        },
        {
            field: 'position', text: 'Position', size: '160px', sortable: true, resizable: true,
            editable: false, clipboardCopy: true
        },
        {
            field: 'progress', text: 'Progress', size: '160px', sortable: true, resizable: true,
            editable: false,
            render(record, extra) {//maybe we can render the image here
                return "<progress value='" + record.progress + "');\"></progress>";
            }
        },
    ],
    onDelete: function (event) {
        //! delete function not net impl yet
        console.log("delete action trig");
        console.log(event);
    },
    onDblClick: function (event) {
        if (event.detail.column == 2) {
            rev(event.detail.recid);
        }
    },
    records: [
    ]
});

let debugMode = false;

let trainlist = new Map();
let tracklist = new Map();
let nodelist = new Map();

let derail_img = new Image();
derail_img.id = "derail-img";
derail_img.src = "derail.png";

window.sendNewTrainRequest = function sendNewTrainRequest(node_id) {
    ctrl_socket.send("train_new\n" + node_id + " " + (Math.random() * 200 + 400));
}

function rev(trainid) {
    console.log("rev trid")
    socket.send("click\n" + trainid + " 0,1,0");
}
function redraw(time) {
    trainlist.forEach((train, id) => {
        if (Number.isNaN(train.movement_start)) {
            if (train.direction == 1) {
                train.movement_start = time - Number(train.start_t) * Number(train.duration);
            } else {
                train.movement_start = time - (1 - Number(train.start_t)) * Number(train.duration);
            }
        }

        let cordlist = tracklist.get(train.track_id).cordlist;
        let current_t = (time - train.movement_start) / train.duration;
        if (train.direction == -1) {
            current_t = 1 - current_t;
        }
        train.current_t = current_t;
        let recordindex = grid_train.get(id, true);
        grid_train.records[recordindex].progress = current_t;


        // if (current_t > 1.1 || current_t < -0.1) {
        //     train.html_row.remove();
        //     trainlist.delete(train.id);
        //     return;
        // }

        let point = bezierPoint(cordlist, current_t);
        let x_pos = point.x;
        let y_pos = point.y;
        train.x = x_pos;
        train.y = y_pos;
        if (Number.isNaN(train.last_pos_time) || time - train.last_pos_time > 200) {
            grid_train.records[recordindex].position = "(" + train.x.toFixed(1).padStart(6, "0")
                + "," + train.y.toFixed(1).padStart(6, "0") + ")";
            train.last_pos_time = time;
        }
        grid_train.update();
        // let dresult = bezierDerivative(cordlist, current_t);
        // let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
    });

    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");
let socket = null;
let ctrl_socket = null;
function startSocket() {
    socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws");
    ctrl_socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + "/ws-ctrl");


    socket.onerror = _ => {
        document.getElementById("status-container").innerText = "";
        document.getElementById("status-container").append(derail_img);
        window.setTimeout(startSocket, 500);
    };

    socket.onopen = (event) => {
        trainlist = new Map();
        tracklist = new Map();
        nodelist = new Map();
        // explosionSerial = 0;
        // explosionList = new Map();




        derail_img.remove();
        document.getElementById("status-container").innerText = "Connected! " + new Date();;

        socket.onmessage = (msg) => {
            if (debugMode) {
                console.log(msg);
            }
            let msg_split = msg.data.split("\n");
            // * ! BLIND start here
            let row_count = 1;
            //prase here
            switch (msg_split[0]) {
                case "train":
                    {
                        let args = msg_split[1].split(" ");
                        let new_train = {};
                        new_train.id = Number(args[0]);
                        new_train.track_id = Number(args[1]);
                        new_train.start_t = Number(args[2]);
                        new_train.duration = Number(args[3]);
                        if (args[4] == "forward") {
                            new_train.direction = 1;
                        } else {
                            new_train.direction = -1;
                        }
                        new_train.img = new Image();
                        new_train.img.src = msg_split[2];
                        new_train.movement_start = NaN;
                        new_train.last_pos_time = NaN;
                        new_train.x = NaN;
                        new_train.y = NaN;

                        let direction;
                        if (args[4] == "forward") {
                            direction = "=>";
                        } else {
                            direction = "<=";
                        }
                        if (grid_train.get(new_train.id) == null) {
                            grid_train.add({
                                recid: new_train.id,
                                track: new_train.track_id,
                                direction: direction,
                                image: new_train.img.src,
                                position: "idk",
                                progress: 0.0
                            });
                        } else {
                            let recordindex = grid_train.get(new_train.id, true);
                            grid_train.records[recordindex].track = new_train.track_id;
                            grid_train.records[recordindex].direction = direction;
                            grid_train.records[recordindex].image = new_train.img.src;
                            grid_train.update();
                            //not sure if rest need update ort not
                        }

                        trainlist.set(Number(args[0]), new_train);
                    }
                    break;
                case "track":
                    for (let i = 2; i < msg_split.length; i++) {
                        let args = msg_split[i].split(" ");
                        let new_track = {};
                        let cordlist = args[1].split(";").map(x => Number(x));
                        cordlist.shift();
                        new_track.id = Number(args[0]);
                        new_track.cordlist = cordlist
                        new_track.color = args[2];
                        new_track.thickness = Number(args[3]);
                        new_track.length = bezierRoughLength(cordlist);

                        if (grid_track.get(new_track.id) == null) {
                            grid_track.add({ recid: new_track.id, start: "(" + cordlist[0] + "," + cordlist[1] + ")", end: "(" + cordlist[cordlist.length - 2] + "," + cordlist[cordlist.length - 1] + ")", color: new_track.color });
                        } else {
                            let recordindex = grid_track.get(new_track.id, true);
                            grid_track.records[recordindex].start = "(" + cordlist[0] + "," + cordlist[1] + ")";
                            grid_track.records[recordindex].end = "(" + cordlist[cordlist.length - 2] + "," + cordlist[cordlist.length - 1] + ")";
                            grid_track.records[recordindex].color = new_track.color;
                            grid_track.update();
                        }
                        tracklist.set(Number(args[0]), new_track);
                    }
                    break;
                case "node":
                    {
                        let args = msg_split[1].split(" ");
                        let new_node = {};
                        new_node.id = Number(args[0]);
                        new_node.x = Number(args[1].split(";")[0]);
                        new_node.y = Number(args[1].split(";")[1]);

                        let new_row = nodelist.get(new_node.id)?.html_row;
                        if (!new_row) {
                            //! Do not support multi control
                            //if not exist
                            if (grid_node.get(new_node.id) == null) {
                                grid_node.add({ recid: new_node.id, PositionX: new_node.x, PositionY: new_node.y, nodetype: "Random" });
                            } else {
                                let recordindex = grid_node.get(new_node.id, true);
                                grid_node.records[recordindex].PositionX = new_node.x;
                                grid_node.records[recordindex].PositionY = new_node.y;
                                grid_node.records[recordindex].nodetype = "Random";
                                grid_node.update();
                            }

                        }


                        nodelist.set(new_node.id, new_node);
                    }
                    break;
                case "remove":
                    let args = msg_split[1].split(" ");
                    // let new_explosion = {};

                    let removed_id = Number(args[0]);
                    // let removal_type = args[1][0];
                    let removed_train = trainlist.get(removed_id);

                    removed_train?.html_row?.remove();
                    trainlist.delete(removed_id);

                    // new_explosion.start = NaN;
                    // new_explosion.x = removed_train.x;
                    // new_explosion.y = removed_train.y;
                    // let cordlist = tracklist.get(removed_train.track_id).cordlist;
                    // new_explosion.dxdy = bezierDerivative(cordlist, removed_train.current_t);
                    // new_explosion.dx = bezierDerivative(cordlist, removed_train.current_t).dx;
                    // new_explosion.dy = bezierDerivative(cordlist, removed_train.current_t).dy;
                    // new_explosion.cordlist = cordlist;
                    // new_explosion.type = removal_type; // e s d v t
                    // new_explosion.train = removed_train;

                    // silent explosion require no further animation
                    // if (removal_type != "s") {
                    //     explosionList.set(explosionSerial, new_explosion);
                    // }
                    // explosionSerial++;

                    break;
            }
        };

        socket.onclose = _ => {
            document.getElementById("status-container").innerText = "";
            document.getElementById("status-container").append(derail_img);
            window.setTimeout(startSocket, 500);
        };
    };
}

startSocket();

// document.getElementById("delete-all").onclick = () => {//TODO: remove for current version
//     trainlist.forEach(train => {
//         socket.send("click\n" + train.id + " 1,0,0");
//     })
// }

let track_select = document.getElementById("track-select");
track_select.addEventListener("click", _ => {
    fetch("/available-tracks").then(response => { return response.json(); })
        .then(list => {
            let track_select = document.getElementById("track-select");
            let current_value = track_select.value;
            track_select.innerHTML = "<option value=\"\">--DEFAULT TRACK--</option>";
            for (let track_name of list) {
                let option = document.createElement("option");
                option.innerHTML = track_name;
                option.value = track_name;
                track_select.append(option);
            }
            track_select.value = current_value;
        });
});
