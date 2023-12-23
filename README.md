# whatdo - CLI project management

```
$ wd
P0
- (user-clean) clean up the user profile [ui]
- (history) implement history
- (scroll-bug) fix horrible scrolling bug [ui]
P1
- ...
$ wd ---priority 1
```

Create branches automatically

```
$ wd start user-clean
$ user-clean>
# user-clean> wd
Current task: clean up the user profile [ui]

P0
- ...
$ user-clean> wd resolve --merge --push
$ master>
```

```
$ wd add fix-bad-alignment
Enter details:
Enter tags:
Enter priority:
$ wd add fix-bad-alignment -m "Alignment is wrong in the right corner of the screen" --tag ui -p 1
```

```
$ wd --tag ui
$ wd user-clean
```
