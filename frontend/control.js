let img = -1;
let image_name = "train_right.png";
const train_width = 350 / 4; // TODO: adjust with image size
const train_height = 263 / 4;

let relative_x = Number(document.cookie.split("; ").find(row => row.startsWith("relative_x="))?.split("=")[1]);
let relative_y = Number(document.cookie.split("; ").find(row => row.startsWith("relative_y="))?.split("=")[1]);

let ask_attempt = 0;

while (Number.isNaN(relative_x) || relative_x < -4000 || 4000 < relative_x) {
    relative_x = Number(window.prompt("Relative x?", "0"));
    ask_attempt++;
    if (ask_attempt > 10) {
        relative_x = 0;
    }
}

ask_attempt = 0;
while (Number.isNaN(relative_y) || relative_y < -2000 || 2000 < relative_y) {
    relative_y = Number(window.prompt("Relative y?", "0"));
    ask_attempt++;
    if (ask_attempt > 10) {
        relative_y = 0;
    }
}

// TODO: ask for x y boundaries and scale track base on view port size
document.cookie = "relative_x=" + relative_x;
document.cookie = "relative_y=" + relative_y;

const main_canvas = document.getElementById("main-canvas");

function resizeCanvas() {
    main_canvas.width = window.innerWidth;
    main_canvas.height = window.innerHeight;
}

window.addEventListener("resize", resizeCanvas);
resizeCanvas();

let derail_img = new Image();
derail_img.src = "derail.png";

let status = "nothing";
let movement_start = 0;
let run_time = 1000.0; //?ms

let trainlist = new Map();
let tracklist = new Map();
let trainposition = [];
let junctionlist = new Map();

function redraw(time) {
    // /**
    // * @param trainlist a list of param including (trainid, trackid)
    // * @param st starting time
    // * @param duration uh the duration?
    // */

    let main_canvas = document.getElementById("main-canvas");
    let main_context = main_canvas.getContext("2d");

    main_context.clearRect(0, 0, main_canvas.width, main_canvas.height);
    main_context.save();
    main_context.translate(relative_x, relative_y);

    if (movement_start == -1) {
        movement_start = time;
    }

    if (img == -1) {//? default image
        if (!image_name) image_name = "train_right.png";
        img = new Image(); // Create new img element
        img.src = image_name;
    }

    tracklist.forEach(track => {
        drawTrack(main_context, track);
    });

    trainposition = [];
    trainlist.forEach((train, id) => {
        if (Number.isNaN(train.movement_start)) {
            train.movement_start = time - Number(train.start_t) * Number(train.duration);
        }

        let cordlist = tracklist.get(train.track_id).cordlist;
        let current_t = (time - train.movement_start) / train.duration;
        if (current_t > 1.1)
            return;
        let point = bezierPoint(cordlist, current_t);
        let x_pos = point.x;
        let y_pos = point.y;
        let trainpositionitem = {};
        trainpositionitem.id = id;
        trainpositionitem.x = x_pos;
        trainpositionitem.y = y_pos;
        trainposition.push(trainpositionitem);
        //! not handling out of bound problem
        // now detrive
        let dresult = bezierDerivative(cordlist, current_t);
        let deg = Math.atan2(dresult.dy, dresult.dx) * 180 / Math.PI;
        drawRotatedImg(main_context, x_pos, y_pos, deg, x_pos - train_width / 2, y_pos - train_height, train.img);
    });

    main_context.restore();
    window.requestAnimationFrame(redraw);
}

window.requestAnimationFrame(redraw);

let url = new URL(window.location.href);
console.log((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");
let socket = new WebSocket((url.protocol == "http:" ? "ws:" : "wss:") + "//" + url.host + url.pathname + "ws");
let maxtrackcount = 0;
socket.onopen = (event) => {
    // socket.send("position\n" + left_bound + " " + right_bound);
    socket.onmessage = (msg) => {
        // console.log(msg);
        let msg_split = msg.data.split("\n");
        // ! BLIND start here
        let row_count = 1;
        //prase here
        switch (msg_split[0]) {
            case "train":
                // code block
                throw new Error("Undefined behavior train update");
            case "track":
                for (i = 2; i < msg_split.length; i++) {
                    args = msg_split[i].split(" ");
                    let track = {};
                    let cordlist = args[1].split(";").map(x => Number(x));
                    cordlist.shift();
                    maxtrackcount = Math.max(maxtrackcount, Number(args[0]));
                    tracklist.set(Number(args[0]), cordlist);
                }
                break;
        }
    };
    socket.onclose = (msg) => {
        main_canvas.hidden = true;
        document.getElementById("main-canvas").parentElement.append(derail_img);
    };
};

// update click on demand
let junction_id = 1; //! MUST CHNAGE THIS TODO

function newnode() {
    let x, y;
    window.addEventListener("click", function (event) { //clicking start point
        mousePos = { x: event.clientX, y: event.clientY };
        //find the closest node -> have a node list? where can i get the node this! -> save all cord when recive to a list
        //find it by time complexity (o(n))
        var validr = 20; //20px this is just a therehold
        tracklist.forEach((id, cordlist) => {
            for (i = 0; i < cordlist.length; i += 2) {
                let realr = Math.sqrt(Math.pow(mousePos.x - cordlist[i], 2) + Math.pow(mousePos.y - cordlist[i + 1], 2));
                if (realr <= validr) {
                    //user is sure clicking this point as a starting point
                    x = cordlist[i];
                    y = cordlist[i + 1];
                    return;
                }
            }
        });
    });

    window.addEventListener("click", function (event) { //clicking end point
        maxtrackcount += 1;//! TODO this function can only add linear line
        socket.send("newnode\n" + junction_id + " " + maxtrackcount + "\n" + x + ";" + y + " " + event.clientX + ";" + event.clientY);
        tracklist.set(maxtrackcount, [x, y, event.clientX, event.clientY]);
    });
}
function SwitchJunction() {
    window.addEventListener("click", function (event) {
    });
}