# Cursed Roon Display Proxy

So, you've got a shiny Roon setup running, and you've seen that awesome
web-based Now Playing page. You're like "hey, I wonder if I could serve
this over the internet", or "I have a PKI infrastructure for my house and
nothing gets served without being trusted, even locally", or something
equally unhinged.

"I know", you thought naively, "I'll just throw it behind HAProxy" (or
nginx, or something), "and then it'll have SSL support". 

Whelp, Roon serves with hard-coded `http://` paths (and the equivalent
`ws://` for the websocket for data).  "Sucks to be me", you thought, "I'd
have to do something really gross to make this work".

Pal, do I have a thing for you.

## Usage

Don't, please don't, I wrote this at like 11pm-2am.

But if you must:

1. Compile this for your platform of choice.  pfSense is gonna be something
   like `x86_64-unknown-freebsd`. Try
   [cargo-cross](https://github.com/cross-rs/cross).

2. Run it on the box that also holds your HAProxy or whatevs, using the
   `ROON_DISPLAY_PROXY_PORT` env var to control what port it listens on (it
   binds to `127.0.0.1`), and `ROON_DISPLAY_BACKEND` to configure the backend
   host-port pair that Roon is listening on (likely `<ipaddr>:9330` or
   `rock:9330` or something similar).

3. Do something gross with HAProxy.  Namely: this won't handle websockets, so
   you'll need to put `/api` (but *just* `/api`, not `/api/xyz`) on a separate
   backend that circumvents this.

   I use something roughly equivalent to:

   ```
    frontend whatsplaying
        bind			<public ip>:443 name <publicip>:443   ssl crt-list /var/etc/haproxy/whatsplaying.crt_list  alpn h2,http/1.1
        mode			http
        acl			whatsplaying	var(txn.txnhost) -m str -i whatsplaying.<mysite>
        acl			whatsplaying-api	var(txn.txnpath) -m beg -i /api
        acl			whatsplaying-root-redir	var(txn.txnpath) -m str -i /

        # other pfsense generated stuff like crt stuff

        http-request set-var(txn.txnhost) hdr(host)
        http-request set-var(txn.txnpath) path
        http-request redirect code 301 prefix display  if  whatsplaying whatsplaying-root-redir aclcrt_whatsplaying
        use_backend roon-display-api_ipvANY  if  whatsplaying whatsplaying-api aclcrt_whatsplaying
        use_backend roon-display_ipvANY  if  whatsplaying !whatsplaying-api aclcrt_whatsplaying

    backend roon-display-api_ipvANY
        mode			http
        id			104
        log			global
        http-check		send meth HEAD uri /display
        timeout connect		30000
        timeout server		30000
        retries			3
        load-server-state-from-file	global
        option			httpchk
        server			roon-rock <rock-ip-address>:9330 id 103 check inter 1000  

    backend roon-display_ipvANY
        mode			http
        id			102
        log			global
        http-check		send meth HEAD uri /display
        timeout connect		30000
        timeout server		30000
        retries			3
        load-server-state-from-file	global
        option			httpchk
        server			local-roon-proxy 127.0.0.1:<this-program-port> id 103 check inter 1000
    ```

## Wut?

Sorry, again, writing this README at like 2am after a long week.

The code is a bit sloppy and uncommented, maybe I'll clean it up later.

## No, I mean, how does this work?

Oh, well, it simply rewrites the request bodies of any GET request with a
content-type of (exactly) `text/html` or `application/x-javascript` (Roon's
mime type for javascript, from last century), replacing `"http://" -->
"https://"` and `"ws://" --> "wss://"`, without looking too closely at anything.
Everything else gets passed through transparently.

Don't make that face at me, what would you propose (when it's 11pm and
you just want to see if you can get this working)?
