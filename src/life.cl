__kernel void life_step(__global uchar* current, __global uchar* next, int width, int height) {
    int x = get_global_id(0);
    int y = get_global_id(1);
    int idx = y * width + x;

    int count = 0;
    for (int dy = -1; dy <= 1; dy++) {
        for (int dx = -1; dx <= 1; dx++) {
            if (dx == 0 && dy == 0) continue;
            int nx = (x + dx + width) % width;
            int ny = (y + dy + height) % height;
            int nidx = ny * width + nx;
            count += current[nidx];
        }
    }

    uchar alive = current[idx];
    uchar next_state = alive;
    if (alive && (count < 2 || count > 3)) {
        next_state = 0;
    } else if (!alive && count == 3) {
        next_state = 1;
    }
    next[idx] = next_state;
}
