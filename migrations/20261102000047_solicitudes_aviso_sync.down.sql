/* Revierte el aviso y la marca de sincronización [28AE-1]. Sin datos que perder:
 * las columnas solo guardan el estado del aviso y la fecha de llegada. */
DROP INDEX IF EXISTS idx_solicitudes_no_vistas;
ALTER TABLE solicitudes DROP COLUMN IF EXISTS sincronizada_en;
ALTER TABLE solicitudes DROP COLUMN IF EXISTS aviso_visto;
