--
-- PostgreSQL database dump
--

-- Dumped from database version 13.3 (Debian 13.3-1.pgdg100+1)
-- Dumped by pg_dump version 13.2

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: documents; Type: TABLE; Schema: public; Owner: imgproxy
--

CREATE TABLE public.documents (
    url_hash character varying(256) NOT NULL,
    url character varying(65536) NOT NULL,
    blocked boolean NOT NULL,
    provider character varying(256) NOT NULL,
    categories character varying(65536),
    doc_hash character varying(256) NOT NULL,
    updated_at timestamp with time zone NOT NULL
);


ALTER TABLE public.documents OWNER TO imgproxy;

--
-- Name: report; Type: TABLE; Schema: public; Owner: imgproxy
--

CREATE TABLE public.report (
    id character varying(512) NOT NULL,
    url character varying(65536) NOT NULL,
    categories character varying(65536),
    url_hash character varying(256),
    updated_at timestamp with time zone NOT NULL,
    apikey character varying(512) NOT NULL,
    ip_addr character varying(512) NOT NULL
);


ALTER TABLE public.report OWNER TO imgproxy;

--
-- Name: documents documents_pkey; Type: CONSTRAINT; Schema: public; Owner: imgproxy
--

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_pkey PRIMARY KEY (url_hash);


--
-- Name: report report_pkey; Type: CONSTRAINT; Schema: public; Owner: imgproxy
--

ALTER TABLE ONLY public.report
    ADD CONSTRAINT report_pkey PRIMARY KEY (id);


--
-- Name: doc_hash_idx; Type: INDEX; Schema: public; Owner: imgproxy
--

CREATE INDEX doc_hash_idx ON public.documents USING btree (doc_hash);


--
-- Name: report_url_hash_idx; Type: INDEX; Schema: public; Owner: imgproxy
--

CREATE INDEX report_url_hash_idx ON public.report USING btree (url_hash);


--
-- Name: url_hash_idx; Type: INDEX; Schema: public; Owner: imgproxy
--

CREATE INDEX url_hash_idx ON public.documents USING btree (url_hash);


--
-- PostgreSQL database dump complete
--

