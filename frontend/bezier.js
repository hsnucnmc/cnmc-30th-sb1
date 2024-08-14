function pointDistance(p1, p2) {
    return Math.sqrt(Math.pow(p1.x - p2.x, 2) + Math.pow(p1.y - p2.y, 2));
}

function bezierPoint(cordlist, current_t) {
    let x_pos = 0, y_pos = 0;
    if (cordlist.length == 4) {
        // straight line
        x_pos = (1 - current_t) * cordlist[0] + current_t * cordlist[2];
        y_pos = (1 - current_t) * cordlist[1] + current_t * cordlist[3];
    } else if (cordlist.length == 6) {
        // one cp
        x_pos = (1 - current_t) * (1 - current_t) * cordlist[0] + 2 * (1 - current_t) * current_t * cordlist[2] + current_t * current_t * cordlist[4];
        y_pos = (1 - current_t) * (1 - current_t) * cordlist[1] + 2 * (1 - current_t) * current_t * cordlist[3] + current_t * current_t * cordlist[5];
    } else if (cordlist.length == 8) {
        // standard 2 cp
        x_pos = (1 - current_t) * (1 - current_t) * (1 - current_t) * cordlist[0] + 3 * (1 - current_t) * (1 - current_t) * current_t * cordlist[2] + 3 * (1 - current_t) * current_t * current_t * cordlist[4] + current_t * current_t * current_t * cordlist[6];
        y_pos = (1 - current_t) * (1 - current_t) * (1 - current_t) * cordlist[1] + 3 * (1 - current_t) * (1 - current_t) * current_t * cordlist[3] + 3 * (1 - current_t) * current_t * current_t * cordlist[5] + current_t * current_t * current_t * cordlist[7];
    }
    return { x: x_pos, y: y_pos };
}

function bezierDerivative(coords, t) {
    const n = (coords.length / 2) - 2; // Number of control points

    if (n === 0) {
        // Straight Line
        const [x0, y0, x1, y1] = coords;

        const invT = 1 - t;

        const dx = x1 - x0;
        const dy = y1 - y0;

        return { dx: dx, dy: dy };
    }
    else if (n === 1) {
        // Quadratic Bezier curve
        const [x0, y0, x1, y1, x2, y2] = coords;

        const invT = 1 - t;

        const dx = 2 * (1 - t) * (x1 - x0) + 2 * t * (x2 - x1);
        const dy = 2 * (1 - t) * (y1 - y0) + 2 * t * (y2 - y1);

        return { dx: dx, dy: dy };
    } else if (n === 2) {
        // Cubic Bezier curve
        const [x0, y0, x1, y1, x2, y2, x3, y3] = coords;

        const invT = 1 - t;
        const invT2 = invT * invT;
        const t2 = t * t;

        const dx = 3 * invT2 * (x1 - x0) + 6 * t * invT * (x2 - x1) + 3 * t2 * (x3 - x2);
        const dy = 3 * invT2 * (y1 - y0) + 6 * t * invT * (y2 - y1) + 3 * t2 * (y3 - y2);

        return { dx: dx, dy: dy };
    } else {
        throw new Error("Unsupported number of control points.\nSupported types are straight lines, quadratic (3 points) and cubic (4 points) beziers.");
    }
}

function bezierSecondDerivative(coords, t) {
    const n = (coords.length / 2) - 2; // Number of control points

    if (n === 0) {
        // Straight Line

        return { ddx: 0, ddy: 0 };
    }
    else if (n === 1) {
        // Quadratic Bezier curve
        const [x0, y0, x1, y1, x2, y2] = coords;

        const invT = 1 - t;

        const dx = 2 * (-1) * (x1 - x0) + 2 * 1 * (x2 - x1);
        const dy = 2 * (-1) * (y1 - y0) + 2 * 1 * (y2 - y1);

        return { ddx: dx, ddy: dy };
    } else if (n === 2) {
        // Cubic Bezier curve
        const [x0, y0, x1, y1, x2, y2, x3, y3] = coords;

        const invT = -1;
        const invT2 = 2 * t;
        const t2 = 2 * t;

        const dx = 3 * invT2 * (x1 - x0) + 6 * (-2) * t * (x2 - x1) + 3 * t2 * (x3 - x2);
        const dy = 3 * invT2 * (y1 - y0) + 6 * (-2) * t * (y2 - y1) + 3 * t2 * (y3 - y2);

        return { ddx: dx, ddy: dy };
    } else {
        throw new Error("Unsupported number of control points.\nSupported types are straight lines, quadratic (3 points) and cubic (4 points) beziers.");
    }
}

function bezierRoughLength(cordlist) {
    let length = 0;
    for (let i = 0; i <= 16; i++) {
        length += pointDistance(bezierPoint(cordlist, i / 16), bezierPoint(cordlist, (i + 1) / 16));
    }

    return length;
}

function radiusOfCurvature(cordlist, current_t) {
    let first = bezierDerivative(cordlist, current_t);
    let second = bezierSecondDerivative(cordlist, current_t);
    let dx = first.dx, dy = first.dy;
    let ddx = second.ddx, ddy = second.ddy;
    return Math.sqrt(dx * dx + dy * dy) / (dx * ddy - dy * ddy);
}
