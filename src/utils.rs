/// Loop for parent path, remve part that matches key path
pub fn find_directory<'a, 'b>(key: &'b str, parent: &'a str) -> Option<&'a str> {
    let (x, mut y) = (0, 1);
    loop {
        if y > parent.len() {
            break None;
        }
        if key == &parent[x..y] {
            break Some(&parent[y..]);
        } else {
            y += 1;
        }
    }
}
